import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

// The sheet's proficiency circle: empty when not proficient, filled when proficient, and
// filled with a ring around it for expertise.
QQC2.AbstractButton {
    id: dot

    // "none", "proficient" or "expertise".
    property string proficiency: "none"

    implicitWidth: Kirigami.Units.gridUnit * 1.2
    implicitHeight: implicitWidth
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    readonly property color ink: hovered || visualFocus ? Kirigami.Theme.focusColor : Kirigami.Theme.textColor

    contentItem: Item {
        Rectangle {
            anchors.centerIn: parent
            width: Math.round(parent.width * 0.62)
            height: width
            radius: width / 2
            color: dot.proficiency === "none" ? "transparent" : dot.ink
            border.color: dot.ink
            border.width: 1.6
        }
        Rectangle {
            visible: dot.proficiency === "expertise"
            anchors.centerIn: parent
            width: parent.width
            height: width
            radius: width / 2
            color: "transparent"
            border.color: dot.ink
            border.width: 1.2
        }
    }
}
