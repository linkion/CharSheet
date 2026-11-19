import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Edits the list of actions at `pointer`, e.g. "/inventory/2/actions". `owner` is "item" or
// "feature" and decides which costs make sense.
ColumnLayout {
    id: editor

    required property string pointer
    property string owner: "item"
    readonly property var actions: App.at(pointer) || []
    readonly property var costOptions: App.catalog.costs.filter(o => owner === "item" ? o.key !== "uses" : (o.key === "none" || o.key === "uses"))

    Layout.fillWidth: true

    Repeater {
        model: editor.actions.length

        Kirigami.AbstractCard {
            id: card
            required property int index
            readonly property var a: editor.actions[index] || ({})
            readonly property string base: editor.pointer + "/" + index

            function save(key, value) {
                App.set(base + "/" + key, value)
            }

            Layout.fillWidth: true

            // The second column takes the width the card is given rather than asking for its
            // contents' natural width, so nothing runs past the dialog's edge; the roll's
            // fields wrap onto a second line when they don't fit.
            contentItem: GridLayout {
                columns: 2

                QQC2.Label { text: "Name:" }
                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    QQC2.TextField {
                        text: card.a.name ?? ""
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        Layout.minimumWidth: Kirigami.Units.gridUnit * 5
                        onEditingFinished: if (text !== card.a.name) card.save("name", text)
                    }
                    QQC2.ComboBox {
                        model: App.catalog.activations
                        textRole: "name"
                        valueRole: "key"
                        currentIndex: (App.catalog.activations).findIndex(o => o.key === card.a.activation)
                        onActivated: card.save("activation", currentValue)
                    }
                    QQC2.ToolButton {
                        icon.name: "edit-delete"
                        onClicked: App.removeAt(editor.pointer, card.index)
                        QQC2.ToolTip.text: "Remove action"
                        QQC2.ToolTip.visible: hovered
                    }
                }
                QQC2.Label { text: "Costs:" }
                RowLayout {
                    QQC2.ComboBox {
                        model: editor.costOptions
                        textRole: "name"
                        valueRole: "key"
                        currentIndex: (editor.costOptions).findIndex(o => o.key === card.a.cost)
                        onActivated: card.save("cost", currentValue)
                    }
                    QQC2.SpinBox {
                        visible: card.a.cost !== "none"
                        from: 1
                        to: 100
                        editable: true
                        value: card.a.cost_amount ?? 1
                        onValueModified: card.save("cost_amount", value)
                    }
                }
                QQC2.Label { text: "Roll:" }
                Flow {
                    spacing: Kirigami.Units.smallSpacing
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    QQC2.TextField {
                        implicitWidth: Kirigami.Units.gridUnit * 7
                        placeholderText: "e.g. 2d4+2"
                        text: card.a.roll ?? ""
                        enabled: !(card.a.roll_formula ?? "")
                        onEditingFinished: if (text !== card.a.roll) card.save("roll", text)
                    }
                    QQC2.TextField {
                        implicitWidth: Kirigami.Units.gridUnit * 8
                        placeholderText: "Formula, e.g. {pb}d6"
                        text: card.a.roll_formula ?? ""
                        onEditingFinished: if (text !== card.a.roll_formula) card.save("roll_formula", text)
                        QQC2.ToolTip.text: "Keeps the roll following the character. Uses {pb}, {level}, {cantrip}, {spell} and {str} to {cha}."
                        QQC2.ToolTip.visible: hovered
                    }
                    QQC2.ComboBox {
                        visible: (card.a.roll ?? "").length > 0
                        model: App.catalog.roll_effects
                        textRole: "name"
                        valueRole: "key"
                        currentIndex: (App.catalog.roll_effects).findIndex(o => o.key === card.a.roll_effect)
                        onActivated: card.save("roll_effect", currentValue)
                    }
                    QQC2.TextField {
                        visible: (card.a.roll ?? "").length > 0
                        implicitWidth: Kirigami.Units.gridUnit * 7
                        placeholderText: "Damage type"
                        text: card.a.damage_type ?? ""
                        onEditingFinished: if (text !== (card.a.damage_type ?? "")) card.save("damage_type", text)
                    }
                }
                QQC2.Label {
                    text: "Description:"
                    Layout.alignment: Qt.AlignTop
                }
                QQC2.TextArea {
                    text: card.a.description ?? ""
                    wrapMode: TextEdit.Wrap
                    placeholderText: "Markdown supported"
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    onEditingFinished: if (text !== card.a.description) card.save("description", text)
                }
                QQC2.Label {
                    text: "Effect:"
                    Layout.alignment: Qt.AlignTop
                }
                EffectEditor {
                    pointer: card.base + "/effect"
                    Layout.preferredWidth: 1
                }
            }
        }
    }

    QQC2.Button {
        text: "Add Action"
        icon.name: "list-add"
        onClicked: App.append(editor.pointer, {})
    }
}
