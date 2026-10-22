import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// A small labelled number; if `rollModifier` is set the number rolls a d20.
Kirigami.AbstractCard {
    id: tile

    property string label
    property string value
    property bool rollable: false
    property int rollModifier

    Layout.fillWidth: true
    Layout.minimumWidth: Kirigami.Units.gridUnit * 5

    contentItem: ColumnLayout {
        spacing: 0
        QQC2.Label {
            text: tile.label
            opacity: 0.7
            font: Kirigami.Theme.smallFont
            horizontalAlignment: Text.AlignHCenter
            Layout.fillWidth: true
            elide: Text.ElideRight
        }
        Kirigami.Heading {
            visible: !tile.rollable
            text: tile.value
            level: 1
            horizontalAlignment: Text.AlignHCenter
            Layout.fillWidth: true
        }
        RollButton {
            visible: tile.rollable
            modifier: tile.rollModifier
            rollLabel: tile.label
            textScale: 1.6
            Layout.alignment: Qt.AlignHCenter
        }
    }
}
