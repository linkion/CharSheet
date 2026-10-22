import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

SheetTab {
    Section {
        title: "Currency"

        GridLayout {
            columns: 2
            Repeater {
                model: [["pp", "Platinum"], ["gp", "Gold"], ["ep", "Electrum"], ["sp", "Silver"], ["cp", "Copper"]]
                delegate: RowLayout {
                    required property var modelData
                    Layout.columnSpan: 2
                    QQC2.Label {
                        text: modelData[1]
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 5
                    }
                    SheetSpin {
                        pointer: "/currency/" + modelData[0]
                        fieldValue: App.c.currency ? App.c.currency[modelData[0]] : 0
                        to: 9999999
                    }
                }
            }
        }
    }

    Section {
        title: "Carrying"

        QQC2.Label {
            text: "Carried: " + (App.d.carried_weight ?? 0).toFixed(1) + " lb of " + (App.d.carrying_capacity ?? 0) + " lb capacity"
        }
        QQC2.ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: App.d.carrying_capacity || 1
            value: Math.min(App.d.carried_weight ?? 0, App.d.carrying_capacity ?? 0)
        }
        QQC2.Label {
            text: "Attuned items: " + (App.d.attuned ?? 0) + " / 3"
            color: (App.d.attuned ?? 0) > 3 ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
        }
    }

    Section {
        title: "Equipment"
        Layout.columnSpan: 2

        headerActions: [
            Kirigami.Action {
                text: "Add Item"
                icon.name: "list-add"
                onTriggered: {
                    if (App.append("/inventory", {}))
                        itemDialog.edit("/inventory", App.c.inventory.length - 1)
                }
            }
        ]

        Repeater {
            model: App.c.inventory

            RowLayout {
                required property var modelData
                required property int index
                Layout.fillWidth: true

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        text: modelData.name + (modelData.quantity !== 1 ? "  ×" + modelData.quantity : "")
                        font.bold: true
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                    QQC2.Label {
                        visible: modelData.notes.length > 0
                        text: modelData.notes
                        opacity: 0.7
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                }
                QQC2.Label {
                    visible: modelData.weight > 0
                    text: (modelData.weight * modelData.quantity).toFixed(1) + " lb"
                    opacity: 0.7
                }
                QQC2.CheckBox {
                    visible: modelData.requires_attunement
                    text: "Attuned"
                    checked: modelData.attuned
                    onToggled: App.set("/inventory/" + index + "/attuned", checked)
                }
                QQC2.ToolButton {
                    icon.name: "document-edit"
                    onClicked: itemDialog.edit("/inventory", index)
                    QQC2.ToolTip.text: "Edit"
                    QQC2.ToolTip.visible: hovered
                }
            }
        }

        QQC2.Label {
            visible: (App.c.inventory || []).length === 0
            text: "Nothing carried yet."
            opacity: 0.7
        }
    }

    ItemDialog {
        id: itemDialog
        title: "Item"
        fields: [
            { key: "name", label: "Name", type: "text" },
            { key: "quantity", label: "Quantity", type: "int", from: 0, to: 9999 },
            { key: "weight", label: "Weight each (lb)", type: "number" },
            { key: "requires_attunement", label: "Requires attunement", type: "bool" },
            { key: "notes", label: "Notes", type: "area" }
        ]
    }
}
