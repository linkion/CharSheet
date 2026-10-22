import QtQuick
import QtQuick.Controls as QQC2
import io.github.porave.charsheet

// A check box bound to a boolean character field.
QQC2.CheckBox {
    required property string pointer
    property bool fieldValue

    checked: fieldValue
    onToggled: App.set(pointer, checked)
}
