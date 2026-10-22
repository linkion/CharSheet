import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import io.github.porave.charsheet

// A multi-line text area bound to a character field, saved when it loses focus.
QQC2.TextArea {
    required property string pointer
    property string value

    text: value
    wrapMode: TextEdit.Wrap
    Layout.fillWidth: true
    Layout.minimumHeight: implicitHeight
    onEditingFinished: if (text !== value) App.set(pointer, text)
}
