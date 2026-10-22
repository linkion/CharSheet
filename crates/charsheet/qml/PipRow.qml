import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import io.github.porave.charsheet

// A row of check boxes for tracking expended resources (slots, uses, death saves), drawn
// as the sheet's diamonds in the decorated themes.
// Checking box i marks i + 1 used; unchecking it marks i used.
RowLayout {
    id: row

    property int total
    property int used
    signal usedEdited(int count)

    spacing: Style.decorated ? 2 : 0

    Repeater {
        model: row.total
        Item {
            id: pip
            required property int index
            readonly property bool used: index < row.used
            function toggled(checked) {
                row.usedEdited(checked ? index + 1 : index)
            }

            implicitWidth: Style.decorated ? diamond.implicitWidth : box.implicitWidth
            implicitHeight: Style.decorated ? diamond.implicitHeight : box.implicitHeight

            DiamondPip {
                id: diamond
                anchors.fill: parent
                visible: Style.decorated
                checked: pip.used
                onToggled: pip.toggled(checked)
            }
            QQC2.CheckBox {
                id: box
                anchors.fill: parent
                visible: !Style.decorated
                checked: pip.used
                onToggled: pip.toggled(checked)
            }
        }
    }
}
