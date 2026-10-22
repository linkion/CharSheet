import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import QtQuick3D
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Throws 3D dice across the window for new rolls, then shows their totals and fades.
// A click anywhere dismisses it and still reaches the sheet underneath.
Item {
    id: overlay

    // The rolls on the table (StoredRoll objects) and their throw (DiceTable.throwRoll).
    property var rolls: []
    property var thrown: null
    property real time: 0
    readonly property real duration: thrown ? Math.max(...thrown.dice.map(d => d.frames.length)) / thrown.fps : 0
    readonly property bool settled: time >= duration

    // World units per simulation unit; Qt Quick 3D's camera expects scenes in the hundreds.
    readonly property real unit: 100

    readonly property color dieColor: Style.mode === "system" ? Kirigami.Theme.highlightColor
        : Style.parchment ? "#7A1F10" : "#383838"
    readonly property color inkColor: Style.mode === "system" ? Kirigami.Theme.highlightedTextColor
        : Style.parchment ? "#F3DFB0" : "#F4F4F4"

    visible: opacity > 0
    opacity: 0

    Behavior on opacity {
        id: fade
        enabled: false
        NumberAnimation {
            duration: Kirigami.Units.longDuration * 2
        }
    }

    // Throws every die of `newRolls` together; they replace whatever is on the table.
    function show(newRolls) {
        // One throw for all of them, so they share the table without passing through each other.
        const combined = { expression: "", mode: newRolls[0].mode, terms: [], total: 0, natural: null, label: "", breakdown: "" }
        for (const roll of newRolls)
            combined.terms.push(...roll.terms)
        const json = DiceTable.throwRoll(JSON.stringify(combined), width / Math.max(1, height))
        thrown = json.length > 0 ? JSON.parse(json) : null
        rolls = newRolls
        time = 0
        hideTimer.stop()
        fade.enabled = false
        opacity = 1
        if (!thrown)
            hideTimer.start()
    }

    function hide() {
        hideTimer.stop()
        fade.enabled = true
        opacity = 0
    }

    onSettledChanged: {
        if (settled && visible)
            hideTimer.restart()
    }

    Timer {
        id: hideTimer
        interval: 3500 + 1000 * Math.max(0, overlay.rolls.length - 1)
        onTriggered: overlay.hide()
    }

    FrameAnimation {
        running: overlay.visible && !overlay.settled
        onTriggered: overlay.time += frameTime
    }

    MouseArea {
        anchors.fill: parent
        onPressed: mouse => {
            overlay.hide()
            mouse.accepted = false
        }
    }

    // Meshes and number textures per kind of die, made when first thrown.
    property var meshes: ({})
    property var atlases: ({})

    function meshFor(shape) {
        if (!meshes[shape]) {
            const m = JSON.parse(DiceTable.mesh(shape))
            const vectors = (a, n) => {
                const out = []
                for (let i = 0; i < a.length; i += n)
                    out.push(n === 3 ? Qt.vector3d(a[i], a[i + 1], a[i + 2]) : Qt.vector2d(a[i], 1 - a[i + 1]))
                return out
            }
            meshes[shape] = {
                raw: m,
                positions: vectors(m.positions, 3),
                normals: vectors(m.normals, 3),
                uvs: vectors(m.uvs, 2),
                indexes: m.indexes
            }
        }
        return meshes[shape]
    }

    function atlasFor(shape) {
        if (!atlases[shape])
            atlases[shape] = atlasTexture.createObject(table, { mesh: meshFor(shape).raw })
        return atlases[shape]
    }

    // New colors after a theme change.
    onDieColorChanged: {
        for (const shape in atlases)
            atlases[shape].destroy()
        atlases = {}
    }

    Component {
        id: atlasTexture
        Texture {
            id: texture
            property var mesh
            generateMipmaps: true
            mipFilter: Texture.Linear
            sourceItem: DieAtlas {
                mesh: texture.mesh
                color: overlay.dieColor
                ink: overlay.inkColor
            }
        }
    }

    View3D {
        anchors.fill: parent
        environment: SceneEnvironment {
            backgroundMode: SceneEnvironment.Transparent
            antialiasingMode: SceneEnvironment.MSAA
            antialiasingQuality: SceneEnvironment.High
        }

        PerspectiveCamera {
            readonly property real tilt: 70
            readonly property real distance: overlay.thrown
                ? 1.08 * overlay.unit * Math.max(overlay.thrown.depth, overlay.thrown.width * height / Math.max(1, width))
                    / Math.tan(fieldOfView * Math.PI / 360)
                : 1000
            fieldOfView: 45
            clipNear: 10
            clipFar: 10000
            eulerRotation.x: -tilt
            position: Qt.vector3d(0, distance * Math.sin(tilt * Math.PI / 180), distance * Math.cos(tilt * Math.PI / 180))
        }

        DirectionalLight {
            eulerRotation: Qt.vector3d(-55, -25, 0)
            brightness: 1.1
        }
        DirectionalLight {
            eulerRotation: Qt.vector3d(-80, 150, 0)
            brightness: 0.35
        }
        // Soft fill from the camera side, so the faces turned away from the key light read.
        PointLight {
            position: Qt.vector3d(0, 1500, 900)
            brightness: 0.4
        }

        Texture {
            id: shadowTexture
            sourceItem: Shape {
                width: 64
                height: 64
                ShapePath {
                    strokeWidth: -1
                    fillGradient: RadialGradient {
                        centerX: 32
                        centerY: 32
                        centerRadius: 32
                        focalX: 32
                        focalY: 32
                        GradientStop { position: 0; color: Qt.rgba(0, 0, 0, 0.45) }
                        GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0) }
                    }
                    PathAngleArc {
                        centerX: 32
                        centerY: 32
                        radiusX: 32
                        radiusY: 32
                        sweepAngle: 360
                    }
                }
            }
        }

        Node {
            id: table
            scale: Qt.vector3d(overlay.unit, overlay.unit, overlay.unit)

            Repeater3D {
                model: overlay.thrown ? overlay.thrown.dice : []

                Node {
                    id: slot
                    required property var modelData

                    Die3D {
                        id: die
                        thrown: slot.modelData
                        fps: overlay.thrown.fps
                        time: overlay.time
                        geometry: overlay.meshFor(slot.modelData.shape)
                        atlas: overlay.atlasFor(slot.modelData.shape)
                    }

                    // A soft shadow on the table, fainter and wider the higher the die.
                    Model {
                        readonly property real lift: Math.max(0, die.position.y)
                        source: "#Rectangle"
                        position: Qt.vector3d(die.position.x + lift * 0.15, 0.01, die.position.z + lift * 0.1)
                        eulerRotation.x: -90
                        scale: Qt.vector3d(0.026 + lift * 0.004, 0.026 + lift * 0.004, 1)
                        opacity: Math.max(0, 1 - lift / 8)
                        materials: PrincipledMaterial {
                            lighting: PrincipledMaterial.NoLighting
                            baseColorMap: shadowTexture
                            alphaMode: PrincipledMaterial.Blend
                            depthDrawMode: Material.NeverDepthDraw
                        }
                    }
                }
            }
        }
    }

    // The totals, once the dice settle.
    Rectangle {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Kirigami.Units.gridUnit * 2
        width: Math.min(parent.width - Kirigami.Units.gridUnit * 2, totals.implicitWidth + Kirigami.Units.gridUnit * 2)
        height: totals.implicitHeight + Kirigami.Units.largeSpacing * 2
        radius: Kirigami.Units.cornerRadius
        color: Style.panel
        border.color: Style.frameOuter
        border.width: 2
        opacity: overlay.settled ? 1 : 0
        scale: overlay.settled ? 1 : 0.9

        Behavior on opacity {
            NumberAnimation { duration: Kirigami.Units.shortDuration }
        }
        Behavior on scale {
            NumberAnimation { duration: Kirigami.Units.shortDuration; easing.type: Easing.OutBack }
        }

        ColumnLayout {
            id: totals
            anchors.centerIn: parent
            width: Math.min(implicitWidth, parent.width - Kirigami.Units.gridUnit * 2)
            spacing: Kirigami.Units.smallSpacing

            Repeater {
                model: overlay.rolls

                RowLayout {
                    id: line
                    required property var modelData
                    readonly property string verdict: modelData.natural === 20 ? "Natural 20!"
                        : modelData.natural === 1 ? "Natural 1" : ""
                    Layout.alignment: Qt.AlignHCenter
                    spacing: Kirigami.Units.largeSpacing

                    Text {
                        text: line.modelData.total
                        font.family: Style.displayFamily
                        font.pixelSize: Kirigami.Units.gridUnit * 2.2
                        color: line.verdict === "Natural 20!" ? Kirigami.Theme.positiveTextColor
                            : line.verdict === "Natural 1" ? Kirigami.Theme.negativeTextColor : Style.heading
                    }
                    ColumnLayout {
                        spacing: 0
                        Text {
                            Layout.fillWidth: true
                            text: line.modelData.label + (line.verdict ? "  — " + line.verdict : "")
                            font.family: Style.labelFamily
                            font.bold: true
                            color: Style.heading
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: line.modelData.breakdown
                            font.family: Style.uiFamily
                            color: Kirigami.Theme.disabledTextColor
                            elide: Text.ElideRight
                        }
                    }
                }
            }
        }
    }
}
