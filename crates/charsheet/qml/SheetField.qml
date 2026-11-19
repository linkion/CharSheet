import QtQuick
import QtQuick.Controls as QQC2
import io.github.linkion.charsheet

// A text field bound to a character field, saved when editing finishes.
QQC2.TextField {
    required property string pointer
    property string value

    text: value
    onEditingFinished: if (text !== value) App.set(pointer, text)
}
