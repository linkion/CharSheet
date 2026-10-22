import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// A small labelled number; if `rollModifier` is set the number rolls a d20.
SheetCard {
    id: tile

    property string label
    property string value
    property bool rollable: false
    property int rollModifier
    // "box", or "shield" (Armor Class) or "oval" (Level) in the decorated themes.
    property string shape: "box"

    Layout.fillWidth: true
    Layout.minimumWidth: Kirigami.Units.gridUnit * 5
    // Tiles in a row share the width equally.
    Layout.preferredWidth: Kirigami.Units.gridUnit * 6
    framed: shape === "box"

    // Width of the shield or oval, which sits centered; the text stays inside it.
    readonly property real shapeWidth: shape === "shield" ? Math.min(width, height * 0.9)
                                     : shape === "oval" ? Math.min(width, height * 0.8) : width
    readonly property bool shaped: Style.decorated && shape !== "box"
    topPadding: shaped ? Kirigami.Units.largeSpacing * 2 : padding
    bottomPadding: shaped && shape === "shield" ? Kirigami.Units.gridUnit * 1.5 : padding

    ShieldShape {
        parent: tile
        z: -1
        visible: Style.decorated && tile.shape === "shield"
        anchors.centerIn: parent
        height: parent.height
        width: tile.shapeWidth
    }
    OvalBadge {
        parent: tile
        z: -1
        visible: Style.decorated && tile.shape === "oval"
        anchors.centerIn: parent
        height: parent.height
        width: tile.shapeWidth
    }

    contentItem: ColumnLayout {
        spacing: 0

        FrameTitle {
            visible: Style.decorated
            text: tile.label
            scale: 1
            rule: tile.shape === "box"
            // Keeps the title inside a shield or oval, wrapping it if need be.
            Layout.maximumWidth: tile.shaped ? tile.shapeWidth - Kirigami.Units.largeSpacing * 2 : -1
            Layout.alignment: Qt.AlignHCenter
        }
        QQC2.Label {
            visible: !Style.decorated
            text: tile.label
            opacity: 0.7
            font: Kirigami.Theme.smallFont
            horizontalAlignment: Text.AlignHCenter
            Layout.fillWidth: true
            elide: Text.ElideRight
        }

        Item {
            Layout.fillWidth: true
            implicitHeight: Math.max(valueLabel.implicitHeight, roll.implicitHeight)
                            + (Style.decorated ? Kirigami.Units.smallSpacing * 2 : 0)

            Flourish {
                anchors.fill: parent
                visible: Style.decorated && tile.shape === "box"
            }
            Kirigami.Heading {
                id: valueLabel
                anchors.centerIn: parent
                visible: !tile.rollable
                text: tile.value
                level: 1
                font.family: Style.displayFamily
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * (Style.decorated ? 2.1 : 1.8)
            }
            RollButton {
                id: roll
                anchors.centerIn: parent
                visible: tile.rollable
                modifier: tile.rollModifier
                rollLabel: tile.label
                textScale: Style.decorated ? 2.1 : 1.6
            }
        }
    }
}
