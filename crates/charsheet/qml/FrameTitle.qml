import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// A box's title: centered capitals over a rule in the decorated themes, a heading otherwise.
ColumnLayout {
    id: title

    property string text
    // Draw the rule under the title (decorated themes only).
    property bool rule: true
    // Size relative to the default font.
    property real scale: 1.15

    spacing: 3
    Layout.fillWidth: true

    QQC2.Label {
        visible: Style.decorated
        text: title.text
        color: Style.heading
        font.family: Style.labelFamily
        font.bold: true
        font.capitalization: Font.AllUppercase
        font.letterSpacing: 0.5
        font.pointSize: Kirigami.Theme.defaultFont.pointSize * title.scale
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
        Layout.fillWidth: true
    }
    Rectangle {
        visible: Style.decorated && title.rule
        implicitHeight: 1.5
        color: Style.frameOuter
        Layout.fillWidth: true
    }
    Kirigami.Heading {
        visible: !Style.decorated
        text: title.text
        level: 3
        elide: Text.ElideRight
        Layout.fillWidth: true
    }
}
