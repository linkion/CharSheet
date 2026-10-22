import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import io.github.porave.charsheet

// A multi-line text area bound to a character field, saved when it loses focus. Written
// on graph paper in the book face in the decorated themes, like the sheet's free-text boxes.
QQC2.TextArea {
    id: area

    required property string pointer
    property string value

    text: value
    wrapMode: TextEdit.Wrap
    font.family: Style.proseFamily
    Layout.fillWidth: true
    Layout.minimumHeight: implicitHeight
    onEditingFinished: if (text !== value) App.set(pointer, text)

    // Inside the background, so the lines sit under the text.
    GridPaper {
        parent: area.background
        anchors.fill: parent
        anchors.margins: 2
    }
}
