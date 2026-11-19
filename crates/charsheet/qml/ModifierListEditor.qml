import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Edits the list of modifiers at `pointer`, e.g. "/inventory/2/modifiers". Only the fields
// that matter for each modifier's kind are shown.
ColumnLayout {
    id: editor

    required property string pointer
    readonly property var modifiers: App.at(pointer) || []
    readonly property var saveOptions: [{ key: null, name: "All saves" }].concat(App.catalog.abilities)

    Layout.fillWidth: true

    // Indexed by count so delegates survive edits elsewhere in the list.
    Repeater {
        model: editor.modifiers.length

        Kirigami.AbstractCard {
            id: card
            required property int index
            readonly property var m: editor.modifiers[index] || ({})
            readonly property string base: editor.pointer + "/" + index
            readonly property bool numeric: ["armor_class", "attack_roll", "save", "speed", "initiative", "max_hp", "damage"].includes(m.kind)

            function save(key, value) {
                App.set(base + "/" + key, value)
            }

            Layout.fillWidth: true

            contentItem: Flow {
                spacing: Kirigami.Units.smallSpacing

                QQC2.ComboBox {
                    model: App.catalog.modifier_kinds
                    textRole: "name"
                    valueRole: "key"
                    currentIndex: (App.catalog.modifier_kinds).findIndex(o => o.key === card.m.kind)
                    onActivated: card.save("kind", currentValue)
                }
                QQC2.SpinBox {
                    visible: card.numeric
                    from: -50
                    to: 50
                    editable: true
                    enabled: !(card.m.value_formula ?? "")
                    value: card.m.value ?? 0
                    onValueModified: card.save("value", value)
                    textFromValue: (v) => (v > 0 ? "+" : "") + v
                    valueFromText: (t) => parseInt(t) || 0
                    QQC2.ToolTip.text: card.m.kind === "damage" ? "Flat damage added to the weapon's damage" : "Bonus"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.TextField {
                    visible: card.numeric
                    implicitWidth: Kirigami.Units.gridUnit * 6
                    placeholderText: "Formula"
                    text: card.m.value_formula ?? ""
                    onEditingFinished: if (text !== card.m.value_formula) card.save("value_formula", text)
                    QQC2.ToolTip.text: "Keeps the bonus following the character, e.g. {pb}. Uses {pb}, {level}, {cantrip}, {spell} and {str} to {cha}."
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.ComboBox {
                    visible: card.m.kind === "attack_roll" || card.m.kind === "damage"
                    model: App.catalog.scopes
                    textRole: "name"
                    valueRole: "key"
                    currentIndex: (App.catalog.scopes).findIndex(o => o.key === card.m.scope)
                    onActivated: card.save("scope", currentValue)
                }
                QQC2.TextField {
                    visible: card.m.kind === "damage"
                    placeholderText: "Dice, e.g. 1d6"
                    text: card.m.dice ?? ""
                    implicitWidth: Kirigami.Units.gridUnit * 6
                    onEditingFinished: if (text !== card.m.dice) card.save("dice", text)
                }
                QQC2.TextField {
                    visible: card.m.kind === "damage" || card.m.kind === "resistance"
                    placeholderText: card.m.kind === "resistance" ? "Type, or all" : "Damage type"
                    text: card.m.damage_type ?? ""
                    implicitWidth: Kirigami.Units.gridUnit * 7
                    onEditingFinished: if (text !== card.m.damage_type) card.save("damage_type", text)
                }
                QQC2.CheckBox {
                    visible: card.m.kind === "damage"
                    text: "Once per turn"
                    checked: card.m.once_per_turn ?? false
                    onToggled: card.save("once_per_turn", checked)
                }
                QQC2.ComboBox {
                    visible: card.m.kind === "save"
                    model: editor.saveOptions
                    textRole: "name"
                    valueRole: "key"
                    currentIndex: (editor.saveOptions).findIndex(o => o.key === (card.m.ability ?? null))
                    onActivated: card.save("ability", currentValue ?? null)
                }
                QQC2.TextField {
                    placeholderText: card.m.kind === "note" ? "e.g. Advantage on Charm saves" : "Label (optional)"
                    text: card.m.text ?? ""
                    implicitWidth: Kirigami.Units.gridUnit * (card.m.kind === "note" ? 16 : 8)
                    onEditingFinished: if (text !== card.m.text) card.save("text", text)
                }
                QQC2.CheckBox {
                    text: "Switchable"
                    checked: card.m.toggleable ?? false
                    onToggled: card.save("toggleable", checked)
                    QQC2.ToolTip.text: "Can be switched on and off from the Combat tab, like a weapon's Fire mode"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.Switch {
                    visible: card.m.toggleable ?? false
                    text: "On"
                    checked: card.m.enabled ?? true
                    onToggled: card.save("enabled", checked)
                }
                QQC2.ToolButton {
                    icon.name: "edit-delete"
                    onClicked: App.removeAt(editor.pointer, card.index)
                    QQC2.ToolTip.text: "Remove modifier"
                    QQC2.ToolTip.visible: hovered
                }
            }
        }
    }

    QQC2.Button {
        text: "Add Modifier"
        icon.name: "list-add"
        onClicked: App.append(editor.pointer, {})
    }
}
