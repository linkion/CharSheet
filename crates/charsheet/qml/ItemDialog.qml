import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Edits one entry of a list (attack, spell, item, feature). Fields are described as
// [{ key, label, type: "text" | "area" | "int" | "number" | "bool" | "choice", options, from, to }]
// and every change is saved immediately.
Kirigami.Dialog {
    id: dialog

    property string listPointer
    property int index: -1
    property var fields: []
    readonly property var item: {
        const list = App.view.character ? listPointer.split("/").filter(p => p).reduce((o, k) => o ? o[k] : undefined, App.c) : undefined
        return list && index >= 0 && index < list.length ? list[index] : ({})
    }

    function edit(pointer, i) {
        listPointer = pointer
        index = i
        open()
    }

    function save(key, value) {
        App.set(listPointer + "/" + index + "/" + key, value)
    }

    standardButtons: Kirigami.Dialog.Close
    preferredWidth: Kirigami.Units.gridUnit * 26
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
                readonly property var fieldValue: dialog.item[modelData.key]

                Kirigami.FormData.label: modelData.type === "bool" ? "" : modelData.label + ":"
                Layout.fillWidth: modelData.type === "text" || modelData.type === "area"

                sourceComponent: {
                    switch (modelData.type) {
                    case "area": return areaField
                    case "int": return intField
                    case "number": return numberField
                    case "bool": return boolField
                    case "choice": return choiceField
                    default: return textField
                    }
                }

                Component {
                    id: textField
                    QQC2.TextField {
                        text: loader.fieldValue ?? ""
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
                        model: loader.modelData.options
                        textRole: "name"
                        valueRole: "key"
                        currentIndex: (loader.modelData.options || []).findIndex(o => o.key === loader.fieldValue)
                        onActivated: dialog.save(loader.modelData.key, currentValue)
                    }
                }
            }
        }
    }
}
