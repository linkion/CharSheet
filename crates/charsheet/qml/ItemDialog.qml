import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Edits one entry of a list (attack, spell, item, feature, effect). Fields are described as
// [{ key, label, type, options, from, to, visibleWhen, owner }] where type is one of
// "text" | "area" | "int" | "number" | "bool" | "choice" | "section" | "modifiers" |
// "actions" | "effect". Keys may be nested ("weapon/damage"), `options` may be a function
// returning the list, and `visibleWhen(entry)` hides a field. Every change is saved immediately.
Kirigami.Dialog {
    id: dialog

    property string listPointer
    property int index: -1
    property var fields: []
    readonly property var item: {
        const list = App.view.character ? App.at(listPointer) : undefined
        return list && index >= 0 && index < list.length ? list[index] : ({})
    }
    readonly property string entryPointer: listPointer + "/" + index

    function edit(pointer, i) {
        listPointer = pointer
        index = i
        open()
    }

    function save(key, value) {
        App.set(entryPointer + "/" + key, value)
    }

    function valueOf(key) {
        return key.split("/").reduce((o, k) => o == null ? undefined : o[k], item)
    }

    standardButtons: Kirigami.Dialog.Close
    preferredWidth: Kirigami.Units.gridUnit * 34
    padding: Kirigami.Units.largeSpacing

    customFooterActions: [
        Kirigami.Action {
            text: "Delete"
            icon.name: "edit-delete"
            onTriggered: {
                App.removeAt(dialog.listPointer, dialog.index)
                dialog.close()
            }
        }
    ]

    Kirigami.FormLayout {
        Repeater {
            model: dialog.fields

            Loader {
                id: loader
                required property var modelData
                readonly property var fieldValue: dialog.valueOf(modelData.key || "")
                readonly property var options: typeof modelData.options === "function" ? modelData.options() : (modelData.options || [])
                readonly property bool editor: ["modifiers", "actions", "effect"].includes(modelData.type)

                visible: !modelData.visibleWhen || modelData.visibleWhen(dialog.item)
                Kirigami.FormData.isSection: modelData.type === "section"
                Kirigami.FormData.label: modelData.type === "bool" ? "" : modelData.type === "section" ? modelData.label : modelData.label + ":"
                Layout.fillWidth: modelData.type === "text" || modelData.type === "area" || editor

                sourceComponent: {
                    switch (modelData.type) {
                    case "area": return areaField
                    case "int": return intField
                    case "number": return numberField
                    case "bool": return boolField
                    case "choice": return choiceField
                    case "section": return sectionField
                    case "modifiers": return modifiersField
                    case "actions": return actionsField
                    case "effect": return effectField
                    default: return textField
                    }
                }

                Component {
                    id: textField
                    QQC2.TextField {
                        text: loader.fieldValue ?? ""
                        placeholderText: loader.modelData.placeholder ?? ""
                        onEditingFinished: if (text !== loader.fieldValue) dialog.save(loader.modelData.key, text)
                    }
                }
                Component {
                    id: areaField
                    QQC2.TextArea {
                        text: loader.fieldValue ?? ""
                        wrapMode: TextEdit.Wrap
                        Layout.minimumHeight: Kirigami.Units.gridUnit * 5
                        onEditingFinished: if (text !== loader.fieldValue) dialog.save(loader.modelData.key, text)
                    }
                }
                Component {
                    id: intField
                    QQC2.SpinBox {
                        from: loader.modelData.from ?? 0
                        to: loader.modelData.to ?? 999
                        editable: true
                        value: loader.fieldValue ?? 0
                        onValueModified: dialog.save(loader.modelData.key, value)
                    }
                }
                Component {
                    id: numberField
                    QQC2.TextField {
                        text: loader.fieldValue ?? 0
                        validator: DoubleValidator { bottom: 0; decimals: 2; notation: DoubleValidator.StandardNotation }
                        onEditingFinished: {
                            const number = parseFloat(text) || 0
                            if (number !== loader.fieldValue)
                                dialog.save(loader.modelData.key, number)
                        }
                    }
                }
                Component {
                    id: boolField
                    QQC2.CheckBox {
                        text: loader.modelData.label
                        checked: loader.fieldValue ?? false
                        onToggled: dialog.save(loader.modelData.key, checked)
                    }
                }
                Component {
                    id: choiceField
                    QQC2.ComboBox {
                        model: loader.options
                        textRole: "name"
                        valueRole: "key"
                        currentIndex: loader.options.findIndex(o => o.key === (loader.fieldValue ?? null))
                        onActivated: dialog.save(loader.modelData.key, currentValue ?? null)
                    }
                }
                Component {
                    id: sectionField
                    Item {}
                }
                Component {
                    id: modifiersField
                    ModifierListEditor {
                        pointer: dialog.entryPointer + "/" + loader.modelData.key
                    }
                }
                Component {
                    id: actionsField
                    ActionListEditor {
                        pointer: dialog.entryPointer + "/" + loader.modelData.key
                        owner: loader.modelData.owner ?? "item"
                    }
                }
                Component {
                    id: effectField
                    EffectEditor {
                        pointer: dialog.entryPointer + "/" + loader.modelData.key
                    }
                }
            }
        }
    }
}
