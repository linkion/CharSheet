import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Adds or edits one entry of a list (attack, spell, item, feature, effect). Fields are described as
// [{ key, label, type, options, from, to, placeholder, visibleWhen, owner }] where type is one of
// "text" | "area" | "int" | "number" | "bool" | "choice" | "flags" | "row" | "section" | "hint" |
// "modifiers" | "actions" | "effect". Keys may be nested ("weapon/damage"), `options` may be a
// function returning the list, and `visibleWhen(entry)` hides a field.
//   flags:   { label, flags: [{ key, label, tip, visibleWhen }] } check boxes in one row.
//   row:     { fields: [...] } short fields side by side, labelled by the first one's label.
//   section: { label, collapsedWhen(entry), badge(entry) } a header that folds away the fields
//            after it, up to the next section; collapsedWhen sets how it starts each time the
//            dialog opens, and badge is shown beside the label, such as count("modifiers").
//   hint:    { label } a line of explanation under a section header.
// Editors, hints and an area without a label take the whole width.
// Every change is saved immediately.
Kirigami.Dialog {
    id: dialog

    property string listPointer
    property int index: -1
    property var fields: []
    // What one entry is called, such as "Item", for the title and buttons.
    property string noun: "Entry"
    // Opened by create(): the entry is new, and Cancel takes it away again.
    property bool creating
    // The new entry as it was added, to tell whether anything was filled in.
    property string pristine
    // Section field index -> folded away.
    property var collapsed: ({})
    property var nameEditor: null
    readonly property var list: App.view.character ? App.at(listPointer) : undefined
    readonly property bool valid: !!list && index >= 0 && index < list.length
    readonly property var item: valid ? list[index] : ({})
    readonly property string entryPointer: listPointer + "/" + index
    // For each field, the index of the section it sits in, or -1 before the first one.
    readonly property var sectionOf: {
        let section = -1
        return fields.map((f, i) => f.type === "section" ? (section = i) : section)
    }
    readonly property real labelWidth: Kirigami.Units.gridUnit * 8

    // Asks every field to save what is typed but not yet saved.
    signal commitRequested()

    function edit(pointer, i) {
        listPointer = pointer
        index = i
        creating = false
        prepare()
        open()
    }

    // Adds an empty entry to the list and opens it for filling in.
    function create(pointer) {
        if (!App.append(pointer, {}))
            return
        listPointer = pointer
        index = App.at(pointer).length - 1
        creating = true
        pristine = JSON.stringify(item)
        prepare()
        open()
    }

    function prepare() {
        const folded = {}
        fields.forEach((f, i) => {
            if (f.type === "section")
                folded[i] = !!(f.collapsedWhen && f.collapsedWhen(item))
        })
        collapsed = folded
    }

    function toggleSection(i) {
        const folded = Object.assign({}, collapsed)
        folded[i] = !folded[i]
        collapsed = folded
    }

    // A section badge counting the entries of one of the entry's lists, as "(2)".
    function count(key) {
        return entry => (entry[key] || []).length > 0 ? "(" + entry[key].length + ")" : ""
    }

    function shown(f) {
        return !f.visibleWhen || !!f.visibleWhen(item)
    }

    function sectionShown(i) {
        return i < 0 || shown(fields[i])
    }

    function fieldShown(f, i) {
        const section = sectionOf[i]
        if (f.type === "section")
            return shown(f)
        return sectionShown(section) && !(section >= 0 && collapsed[section]) && shown(f)
    }

    function save(key, value) {
        App.set(entryPointer + "/" + key, value)
    }

    function valueOf(key) {
        return key.split("/").reduce((o, k) => o == null ? undefined : o[k], item)
    }

    function finish() {
        commitRequested()
        creating = false
        close()
    }

    function discard() {
        if (creating && valid)
            App.removeAt(listPointer, index)
        creating = false
        close()
    }

    // Deleting the entry, as from the MCP server, leaves nothing to edit.
    onValidChanged: if (!valid && opened) close()
    onOpened: if (creating && nameEditor) Qt.callLater(nameEditor.focusControl)
    // Closed some other way (Escape, the corner button): keep a new entry only if something was
    // filled in, so blank rows don't pile up.
    onClosed: {
        if (!creating)
            return
        commitRequested()
        if (valid && JSON.stringify(item) === pristine)
            App.removeAt(listPointer, index)
        creating = false
    }

    title: creating ? "New " + noun : (item.name || noun)
    standardButtons: Kirigami.Dialog.NoButton
    // Follows the window, within a width where the forms and editors read comfortably.
    preferredWidth: Math.min(absoluteMaximumWidth,
        Math.max(Kirigami.Units.gridUnit * 32, Math.min(Kirigami.Units.gridUnit * 44, (parent?.width ?? 0) * 0.6)))
    maximumWidth: preferredWidth
    padding: Kirigami.Units.largeSpacing

    customFooterActions: [
        Kirigami.Action {
            text: "Delete"
            icon.name: "edit-delete"
            visible: !dialog.creating
            onTriggered: confirmDelete.open()
        },
        Kirigami.Action {
            text: "Cancel"
            icon.name: "dialog-cancel"
            visible: dialog.creating
            onTriggered: dialog.discard()
        },
        Kirigami.Action {
            text: dialog.creating ? "Add " + dialog.noun : "Done"
            icon.name: dialog.creating ? "list-add" : "dialog-ok"
            onTriggered: dialog.finish()
        }
    ]

    // Reports no width of its own, so a long line typed into a text area can't widen the
    // dialog or let it scroll sideways: it takes the width the dialog hands it.
    Item {
        implicitWidth: 0
        implicitHeight: form.implicitHeight

        ColumnLayout {
            id: form
            width: parent.width
            spacing: Kirigami.Units.smallSpacing

            Repeater {
                model: dialog.fields

                Loader {
                    id: slot
                    required property var modelData
                    required property int index
                    readonly property bool wide: ["modifiers", "actions", "effect", "hint"].includes(modelData.type)
                                                 || (modelData.type === "area" && !modelData.label)

                    visible: dialog.fieldShown(modelData, index)
                    Layout.fillWidth: true
                    Layout.topMargin: modelData.type === "section" ? Kirigami.Units.largeSpacing : 0
                    sourceComponent: modelData.type === "section" ? sectionHeader : modelData.type === "hint" ? hint : wide ? wideField : labeledField

                    Component {
                        id: sectionHeader
                        ColumnLayout {
                            id: headerColumn
                            readonly property bool folded: !!dialog.collapsed[slot.index]
                            spacing: Kirigami.Units.smallSpacing

                            QQC2.AbstractButton {
                                id: header
                                Layout.fillWidth: true
                                onClicked: dialog.toggleSection(slot.index)
                                Accessible.name: slot.modelData.label

                                HoverHandler { cursorShape: Qt.PointingHandCursor }

                                contentItem: RowLayout {
                                    spacing: Kirigami.Units.smallSpacing
                                    Kirigami.Icon {
                                        source: headerColumn.folded ? "go-next-symbolic" : "go-down-symbolic"
                                        implicitWidth: Kirigami.Units.iconSizes.small
                                        implicitHeight: Kirigami.Units.iconSizes.small
                                    }
                                    Kirigami.Heading {
                                        text: slot.modelData.label
                                        level: 4
                                        color: header.hovered ? Kirigami.Theme.highlightColor : Kirigami.Theme.textColor
                                    }
                                    QQC2.Label {
                                        readonly property string badge: slot.modelData.badge ? slot.modelData.badge(dialog.item) : ""
                                        visible: badge.length > 0
                                        text: badge
                                        opacity: 0.6
                                    }
                                    Item { Layout.fillWidth: true }
                                }
                            }
                            Kirigami.Separator {
                                Layout.fillWidth: true
                            }
                        }
                    }

                    Component {
                        id: hint
                        QQC2.Label {
                            text: slot.modelData.label
                            wrapMode: Text.WordWrap
                            opacity: 0.7
                        }
                    }

                    // Editors of lists (modifiers, actions, an effect) take the whole width.
                    Component {
                        id: wideField
                        FieldEditor {
                            field: slot.modelData
                            form: dialog
                        }
                    }

                    // A label in the left column, and one control, or several side by side.
                    Component {
                        id: labeledField
                        RowLayout {
                            id: line
                            readonly property var parts: slot.modelData.type === "row" ? slot.modelData.fields : [slot.modelData]
                            function fills(f) {
                                return ["text", "area", "choice", "flags"].includes(f.type)
                            }
                            spacing: Kirigami.Units.largeSpacing

                            QQC2.Label {
                                text: slot.modelData.type === "bool" || !line.parts[0].label ? "" : line.parts[0].label + ":"
                                horizontalAlignment: Text.AlignRight
                                wrapMode: Text.WordWrap
                                Layout.preferredWidth: dialog.labelWidth
                                Layout.alignment: slot.modelData.type === "area" ? Qt.AlignTop : Qt.AlignVCenter
                                Layout.topMargin: slot.modelData.type === "area" ? Kirigami.Units.smallSpacing : 0
                            }

                            Repeater {
                                model: line.parts

                                RowLayout {
                                    id: part
                                    required property var modelData
                                    required property int index
                                    visible: dialog.shown(modelData)
                                    spacing: Kirigami.Units.smallSpacing
                                    Layout.fillWidth: line.fills(modelData)
                                    Layout.leftMargin: index > 0 ? Kirigami.Units.largeSpacing : 0

                                    QQC2.Label {
                                        visible: part.index > 0 && part.modelData.type !== "bool"
                                        text: part.modelData.label + ":"
                                    }
                                    FieldEditor {
                                        id: editor
                                        field: part.modelData
                                        form: dialog
                                        Layout.fillWidth: line.fills(part.modelData)
                                        Component.onCompleted: if (part.modelData.key === "name") dialog.nameEditor = editor
                                    }
                                }
                            }

                            Item {
                                visible: !line.parts.some(f => line.fills(f))
                                Layout.fillWidth: true
                            }
                        }
                    }
                }
            }
        }
    }

    Kirigami.PromptDialog {
        id: confirmDelete
        parent: dialog.parent
        title: "Delete " + (dialog.item.name || "this " + dialog.noun.toLowerCase()) + "?"
        subtitle: "It's removed from the sheet along with everything set on it."
        standardButtons: Kirigami.Dialog.Cancel
        customFooterActions: [
            Kirigami.Action {
                text: "Delete"
                icon.name: "edit-delete"
                onTriggered: {
                    confirmDelete.close()
                    App.removeAt(dialog.listPointer, dialog.index)
                    dialog.close()
                }
            }
        ]
    }
}
