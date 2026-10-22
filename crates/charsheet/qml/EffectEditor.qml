import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Edits the optional effect at `pointer` (an action's or spell's), which is applied to the
// character when it's used.
ColumnLayout {
    id: editor

    required property string pointer
    readonly property var effect: App.at(pointer)

    function save(key, value) {
        App.set(pointer + "/" + key, value)
    }

    Layout.fillWidth: true

    QQC2.Button {
        visible: !editor.effect
        text: "Add an Effect"
        icon.name: "list-add"
        onClicked: App.set(editor.pointer, {})
        QQC2.ToolTip.text: "Something applied to you when this is used, such as resistance or a damage rider"
        QQC2.ToolTip.visible: hovered
    }

    GridLayout {
        visible: !!editor.effect
        columns: 2
        Layout.fillWidth: true

        QQC2.Label { text: "Name:" }
        QQC2.TextField {
            text: editor.effect?.name ?? ""
            Layout.fillWidth: true
            onEditingFinished: if (text !== editor.effect.name) editor.save("name", text)
        }
        QQC2.Label { text: "Lasts:" }
        RowLayout {
            QQC2.SpinBox {
                from: 0
                to: 1000
                editable: true
                value: editor.effect?.rounds ?? 0
                onValueModified: editor.save("rounds", value)
            }
            QQC2.Label {
                text: (editor.effect?.rounds ?? 0) === 0 ? "until removed" : "turns (ends at the start of your turn)"
                opacity: 0.7
            }
        }
        Item { width: 1 }
        QQC2.CheckBox {
            text: "Concentration"
            checked: editor.effect?.concentration ?? false
            onToggled: editor.save("concentration", checked)
        }
        QQC2.Label { text: "Notes:" }
        QQC2.TextField {
            text: editor.effect?.notes ?? ""
            Layout.fillWidth: true
            onEditingFinished: if (text !== editor.effect.notes) editor.save("notes", text)
        }
    }

    ModifierListEditor {
        visible: !!editor.effect
        pointer: editor.pointer + "/modifiers"
    }

    QQC2.Button {
        visible: !!editor.effect
        text: "Remove Effect"
        icon.name: "edit-delete"
        onClicked: App.set(editor.pointer, null)
    }
}
