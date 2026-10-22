import QtQuick
import QtQuick.Controls as QQC2
import io.github.porave.charsheet

// A spin box bound to an integer character field.
QQC2.SpinBox {
    required property string pointer
    property int fieldValue

    value: fieldValue
    editable: true
    from: 0
    to: 999
    onValueModified: App.set(pointer, value)
}
