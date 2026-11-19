import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// The control for one field of an ItemDialog (see there for the field format), bound to the
// entry's value and saving every change straight away.
Loader {
    id: loader

    required property var field
    // The ItemDialog the field belongs to: provides item, valueOf(), save() and entryPointer.
    required property var form
    readonly property var fieldValue: form.valueOf(field.key || "")
    readonly property var options: typeof field.options === "function" ? field.options() : (field.options || [])

    // Saves what is typed but not yet saved, as when the dialog closes from the keyboard.
    function commit() {
        if (item && item.commit)
            item.commit()
    }

    // Focuses the control; a text field's words are selected, so typing replaces a default name.
    function focusControl() {
        if (!item)
            return
        item.forceActiveFocus()
        if (item.selectAll)
            item.selectAll()
    }


    Connections {
        target: loader.form
        function onCommitRequested() {
            loader.commit()
        }
    }

    sourceComponent: {
        switch (field.type) {
        case "area": return areaField
        case "int": return intField
        case "number": return numberField
        case "bool": return boolField
        case "choice": return choiceField
        case "flags": return flagsField
        case "modifiers": return modifiersField
        case "actions": return actionsField
        case "effect": return effectField
        default: return textField
        }
    }

    Component {
        id: textField
        QQC2.TextField {
            function commit() {
                if (text !== (loader.fieldValue ?? ""))
                    loader.form.save(loader.field.key, text)
            }
            text: loader.fieldValue ?? ""
            placeholderText: loader.field.placeholder ?? ""
            onEditingFinished: commit()
        }
    }
    Component {
        id: areaField
        QQC2.TextArea {
            function commit() {
                if (text !== (loader.fieldValue ?? ""))
                    loader.form.save(loader.field.key, text)
            }
            text: loader.fieldValue ?? ""
            wrapMode: TextEdit.Wrap
            placeholderText: loader.field.placeholder ?? "Markdown supported"
            // Room for a paragraph even when empty, growing with what is written.
            implicitHeight: Math.max(Kirigami.Units.gridUnit * 6, contentHeight + topPadding + bottomPadding)
            onEditingFinished: commit()
        }
    }
    Component {
        id: intField
        QQC2.SpinBox {
            from: loader.field.from ?? 0
            to: loader.field.to ?? 999
            editable: true
            value: loader.fieldValue ?? 0
            onValueModified: loader.form.save(loader.field.key, value)
        }
    }
    Component {
        id: numberField
        QQC2.TextField {
            function commit() {
                const number = parseFloat(text) || 0
                if (number !== loader.fieldValue)
                    loader.form.save(loader.field.key, number)
            }
            text: loader.fieldValue ?? 0
            validator: DoubleValidator { bottom: 0; decimals: 2; notation: DoubleValidator.StandardNotation }
            Layout.preferredWidth: Kirigami.Units.gridUnit * 5
            onEditingFinished: commit()
        }
    }
    Component {
        id: boolField
        QQC2.CheckBox {
            text: loader.field.label
            checked: loader.fieldValue ?? false
            onToggled: loader.form.save(loader.field.key, checked)
        }
    }
    Component {
        id: choiceField
        QQC2.ComboBox {
            model: loader.options
            textRole: "name"
            valueRole: "key"
            currentIndex: loader.options.findIndex(o => o.key === (loader.fieldValue ?? null))
            onActivated: loader.form.save(loader.field.key, currentValue ?? null)
        }
    }
    // Several yes/no switches of the entry in one wrapping row.
    Component {
        id: flagsField
        Flow {
            spacing: Kirigami.Units.largeSpacing
            Repeater {
                model: loader.field.flags
                QQC2.CheckBox {
                    required property var modelData
                    visible: !modelData.visibleWhen || !!modelData.visibleWhen(loader.form.item)
                    text: modelData.label
                    checked: loader.form.valueOf(modelData.key) ?? false
                    onToggled: loader.form.save(modelData.key, checked)
                    QQC2.ToolTip.text: modelData.tip ?? ""
                    QQC2.ToolTip.visible: hovered && !!modelData.tip
                }
            }
        }
    }
    Component {
        id: modifiersField
        ModifierListEditor {
            pointer: loader.form.entryPointer + "/" + loader.field.key
        }
    }
    Component {
        id: actionsField
        ActionListEditor {
            pointer: loader.form.entryPointer + "/" + loader.field.key
            owner: loader.field.owner ?? "item"
        }
    }
    Component {
        id: effectField
        EffectEditor {
            pointer: loader.form.entryPointer + "/" + loader.field.key
        }
    }
}
