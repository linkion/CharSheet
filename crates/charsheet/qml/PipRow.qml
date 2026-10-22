import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts

// A row of check boxes for tracking expended resources (slots, uses, death saves).
// Checking box i marks i + 1 used; unchecking it marks i used.
RowLayout {
    id: row

    property int total
    property int used
    signal usedEdited(int count)

    spacing: 0

    Repeater {
        model: row.total
        QQC2.CheckBox {
            required property int index
            checked: index < row.used
            onToggled: row.usedEdited(checked ? index + 1 : index)
        }
    }
}
