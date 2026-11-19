import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// One ability as the character sheet lays it out: the modifier in a circle with the score
// in a tab beside it, then the saving throw and the skills that use the ability.
SheetCard {
    id: box

    // An entry of App.d.abilities.
    required property var ability
    // The entries of App.d.skills that use this ability.
    property var skills: []

    readonly property var nextProficiency: ({ none: "proficient", proficient: "expertise", expertise: "none" })
    readonly property var proficiencyNames: ({ none: "Not proficient", proficient: "Proficient", expertise: "Expertise" })

    Layout.fillWidth: true
    Layout.alignment: Qt.AlignTop

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        FrameTitle {
            text: box.ability.name
            rule: false
        }

        // Modifier circle and score tab (decorated themes).
        Item {
            id: dial
            visible: Style.decorated
            Layout.fillWidth: true
            implicitHeight: diameter + caption.implicitHeight + 2

            readonly property real diameter: Kirigami.Units.gridUnit * 4
            readonly property real tabWidth: diameter * 0.7
            readonly property real tabHeight: diameter * 0.56
            readonly property real startX: (width - diameter - tabWidth) / 2
            readonly property real cy: diameter / 2

            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: Style.frameOuter
                    strokeWidth: 1.6
                    fillColor: Style.panel
                    joinStyle: ShapePath.MiterJoin
                    PathSvg {
                        readonly property real x0: dial.startX + dial.diameter / 2
                        readonly property real x1: dial.startX + dial.diameter + dial.tabWidth
                        readonly property real t: dial.cy - dial.tabHeight / 2
                        readonly property real b: dial.cy + dial.tabHeight / 2
                        path: `M ${x0} ${t} L ${x1 - 9} ${t} L ${x1} ${t + 9} L ${x1} ${b} L ${x0} ${b} Z`
                    }
                }
            }
            Rectangle {
                x: dial.startX
                width: dial.diameter
                height: width
                radius: width / 2
                color: Style.panel
                border.color: Style.frameOuter
                border.width: 2.5
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 5
                    radius: width / 2
                    color: "transparent"
                    border.color: Style.frameInner
                    border.width: 1
                }
                RollButton {
                    anchors.centerIn: parent
                    modifier: box.ability.modifier
                    target: ({ kind: "ability", ability: box.ability.key })
                    rollLabel: box.ability.name + " check"
                    parts: box.ability.modifier_parts
                    textScale: 2.3
                }
            }
            QQC2.Label {
                x: dial.startX + dial.diameter + 2
                width: dial.tabWidth - 4
                y: dial.cy - height / 2
                text: box.ability.score
                font.family: Style.displayFamily
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.6
                horizontalAlignment: Text.AlignHCenter

                HoverHandler {
                    id: scoreHover
                }
                BreakdownTip {
                    visible: scoreHover.hovered
                    parts: box.ability.score_parts
                    total: box.ability.score
                    title: box.ability.name + " score"
                }
            }
            QQC2.Label {
                id: caption
                x: dial.startX
                width: dial.diameter
                y: dial.diameter + 2
                text: "Modifier"
                font.family: Style.labelFamily
                font.capitalization: Font.AllUppercase
                font.letterSpacing: 1
                font.pointSize: Kirigami.Theme.smallFont.pointSize
                horizontalAlignment: Text.AlignHCenter
            }
            QQC2.Label {
                x: dial.startX + dial.diameter
                width: dial.tabWidth
                y: dial.cy + dial.tabHeight / 2 + 2
                text: "Score"
                font: caption.font
                horizontalAlignment: Text.AlignHCenter
            }
        }

        // Plain modifier and score (System theme).
        ColumnLayout {
            visible: !Style.decorated
            spacing: 0
            Layout.fillWidth: true
            RollButton {
                modifier: box.ability.modifier
                target: ({ kind: "ability", ability: box.ability.key })
                rollLabel: box.ability.name + " check"
                parts: box.ability.modifier_parts
                textScale: 1.8
                Layout.alignment: Qt.AlignHCenter
            }
            QQC2.Label {
                text: box.ability.score
                opacity: 0.7
                horizontalAlignment: Text.AlignHCenter
                Layout.fillWidth: true

                HoverHandler {
                    id: plainScoreHover
                }
                BreakdownTip {
                    visible: plainScoreHover.hovered
                    parts: box.ability.score_parts
                    total: box.ability.score
                    title: box.ability.name + " score"
                }
            }
        }

        Rectangle {
            implicitHeight: 1
            color: Style.decorated ? Style.frameOuter : Kirigami.Theme.disabledTextColor
            opacity: Style.decorated ? 1 : 0.4
            Layout.fillWidth: true
        }

        RowLayout {
            Layout.fillWidth: true
            ProficiencyDot {
                proficiency: box.ability.save_proficient ? "proficient" : "none"
                enabled: !box.ability.save_from_class
                opacity: enabled ? 1 : 0.6
                onClicked: {
                    const saves = (App.c.extra_saves || []).filter(a => a !== box.ability.key)
                    if (!box.ability.save_proficient)
                        saves.push(box.ability.key)
                    App.set("/extra_saves", saves)
                }
                QQC2.ToolTip.text: box.ability.save_from_class ? "Proficient from your starting class" : box.proficiencyNames[proficiency]
                QQC2.ToolTip.visible: hovered
            }
            RollButton {
                modifier: box.ability.save
                target: ({ kind: "save", ability: box.ability.key })
                rollLabel: box.ability.name + " save"
                parts: box.ability.save_parts
                showBreakdown: saveHover.hovered
            }
            QQC2.Label {
                text: "Saving Throw"
                font.bold: true
                elide: Text.ElideRight
                Layout.fillWidth: true
                HoverHandler {
                    id: saveHover
                }
            }
        }

        Rectangle {
            visible: box.skills.length > 0
            implicitHeight: 1
            color: Style.decorated ? Style.frameOuter : Kirigami.Theme.disabledTextColor
            opacity: Style.decorated ? 1 : 0.4
            Layout.fillWidth: true
        }

        Repeater {
            model: box.skills

            RowLayout {
                required property var modelData
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                ProficiencyDot {
                    proficiency: modelData.proficiency
                    onClicked: App.set("/skills/" + modelData.key, box.nextProficiency[modelData.proficiency])
                    QQC2.ToolTip.text: box.proficiencyNames[modelData.proficiency] + " — click to change"
                    QQC2.ToolTip.visible: hovered
                }
                RollButton {
                    modifier: modelData.modifier
                    target: ({ kind: "skill", skill: modelData.key })
                    rollLabel: modelData.name
                    parts: modelData.parts
                    showBreakdown: skillHover.hovered
                }
                QQC2.Label {
                    text: modelData.name
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                    HoverHandler {
                        id: skillHover
                    }
                }
            }
        }
    }
}
