import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Searches the SRD and adds what's chosen to the sheet. `categories` are the kinds of record a
// tab can hold, as [{ key, name }]. Stays open after adding, as spells tend to come in groups.
Kirigami.Dialog {
    id: dialog

    property var categories: []
    property var enabledCategories: []
    property var results: []
    property int total: 0
    property var hit: null
    property var open_: null
    property var preview: null
    property string base
    property string variant
    property string choice
    property var equipped: null
    property bool attuned

    readonly property var entry: preview?.entry ?? null
    readonly property bool canAdd: !!entry
    readonly property bool isItem: entry?.kind !== undefined

    function show() {
        enabledCategories = []
        search.text = ""
        runSearch()
        open()
        search.forceActiveFocus()
    }

    function runSearch() {
        const keys = enabledCategories.length > 0 ? enabledCategories : categories.map(c => c.key)
        const found = JSON.parse(Srd.search(search.text, JSON.stringify(keys), 100))
        results = found.results
        total = found.total
        select(results.length > 0 ? results[0] : null)
    }

    function select(next) {
        hit = next
        open_ = next ? JSON.parse(Srd.options(next.id)) : null
        base = next?.base ?? (open_?.bases.length === 1 ? open_.bases[0].id : "")
        variant = next?.variant ?? ""
        choice = ""
        equipped = null
        attuned = false
        refreshPreview()
    }

    function spec() {
        const s = { id: hit.id }
        if (base) s.base = base
        if (variant) s.variant = variant
        if (choice) s.choice = choice
        if (equipped !== null) s.equipped = equipped
        if (attuned) s.attuned = true
        return s
    }

    function refreshPreview() {
        preview = hit ? JSON.parse(App.sheet.previewSrd(JSON.stringify(spec()))) : null
    }

    function add() {
        const json = App.sheet.addFromSrd(JSON.stringify(spec()))
        if (!json)
            return
        const added = JSON.parse(json)
        App.report(["Added " + added.entry.name].concat(added.warnings).join(" · "))
        refreshPreview()
    }

    standardButtons: Kirigami.Dialog.Close
    preferredWidth: Kirigami.Units.gridUnit * 50
    preferredHeight: Kirigami.Units.gridUnit * 34
    padding: Kirigami.Units.largeSpacing

    customFooterActions: [
        Kirigami.Action {
            text: "Add"
            icon.name: "list-add"
            enabled: dialog.canAdd
            onTriggered: dialog.add()
        }
    ]

    Timer {
        id: debounce
        interval: 120
        onTriggered: dialog.runSearch()
    }

    ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        Kirigami.SearchField {
            id: search
            placeholderText: "Search the compendium…"
            Layout.fillWidth: true
            onTextChanged: debounce.restart()
        }

        Flow {
            visible: dialog.categories.length > 1
            spacing: Kirigami.Units.smallSpacing
            Layout.fillWidth: true
            Repeater {
                model: dialog.categories
                Kirigami.Chip {
                    required property var modelData
                    text: modelData.name
                    checkable: true
                    closable: false
                    checked: dialog.enabledCategories.includes(modelData.key)
                    onToggled: {
                        const others = dialog.enabledCategories.filter(k => k !== modelData.key)
                        dialog.enabledCategories = checked ? others.concat([modelData.key]) : others
                        dialog.runSearch()
                    }
                }
            }
        }

        RowLayout {
            spacing: Kirigami.Units.largeSpacing
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: list
                clip: true
                model: dialog.results
                currentIndex: dialog.results.indexOf(dialog.hit)
                Layout.preferredWidth: Kirigami.Units.gridUnit * 18
                Layout.fillHeight: true
                Layout.preferredHeight: Kirigami.Units.gridUnit * 24
                QQC2.ScrollBar.vertical: QQC2.ScrollBar {}

                delegate: QQC2.ItemDelegate {
                    id: row
                    required property var modelData
                    required property int index
                    width: ListView.view.width - Kirigami.Units.largeSpacing
                    highlighted: ListView.isCurrentItem
                    onClicked: dialog.select(modelData)
                    contentItem: ColumnLayout {
                        spacing: 0
                        QQC2.Label {
                            text: modelData.name
                            font.bold: true
                            color: row.highlighted ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        QQC2.Label {
                            text: modelData.facets.slice(0, 3).join(" · ")
                            visible: text.length > 0
                            opacity: 0.7
                            font: Kirigami.Theme.smallFont
                            color: row.highlighted ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                    }
                }

                QQC2.Label {
                    anchors.centerIn: parent
                    visible: dialog.results.length === 0
                    text: "Nothing found"
                    opacity: 0.7
                }
            }

            QQC2.ScrollView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredHeight: Kirigami.Units.gridUnit * 24
                contentWidth: availableWidth

                ColumnLayout {
                    width: parent.width
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Heading {
                        text: dialog.hit?.name ?? ""
                        level: 3
                        wrapMode: Text.Wrap
                        Layout.fillWidth: true
                    }

                    GridLayout {
                        columns: 2
                        Layout.fillWidth: true

                        QQC2.Label {
                            text: "Base:"
                            visible: baseBox.visible
                        }
                        QQC2.ComboBox {
                            id: baseBox
                            visible: (dialog.open_?.bases.length ?? 0) > 1
                            model: dialog.open_?.bases ?? []
                            textRole: "name"
                            valueRole: "id"
                            displayText: currentIndex < 0 ? "Choose…" : currentText
                            currentIndex: (dialog.open_?.bases ?? []).findIndex(b => b.id === dialog.base)
                            Layout.fillWidth: true
                            onActivated: {
                                dialog.base = currentValue
                                dialog.refreshPreview()
                            }
                        }
                        QQC2.Label {
                            text: "Variant:"
                            visible: variantBox.visible
                        }
                        QQC2.ComboBox {
                            id: variantBox
                            visible: (dialog.open_?.variants.length ?? 0) > 0
                            model: dialog.open_?.variants ?? []
                            displayText: currentIndex < 0 ? "Choose…" : currentText
                            currentIndex: (dialog.open_?.variants ?? []).indexOf(dialog.variant)
                            Layout.fillWidth: true
                            onActivated: {
                                dialog.variant = currentText
                                dialog.refreshPreview()
                            }
                        }
                        QQC2.Label {
                            text: (dialog.open_?.choice?.what ?? "") + ":"
                            visible: choiceBox.visible
                        }
                        QQC2.ComboBox {
                            id: choiceBox
                            visible: !!dialog.open_?.choice
                            model: dialog.open_?.choice?.allowed ?? []
                            displayText: currentIndex < 0 ? "Choose…" : currentText
                            currentIndex: (dialog.open_?.choice?.allowed ?? []).indexOf(dialog.choice)
                            Layout.fillWidth: true
                            onActivated: {
                                dialog.choice = currentText
                                dialog.refreshPreview()
                            }
                        }
                    }

                    RowLayout {
                        visible: dialog.isItem
                        QQC2.CheckBox {
                            text: "Equipped"
                            checked: dialog.entry?.equipped ?? false
                            onToggled: {
                                dialog.equipped = checked
                                dialog.refreshPreview()
                            }
                        }
                        QQC2.CheckBox {
                            text: "Attuned"
                            visible: dialog.entry?.requires_attunement ?? false
                            checked: dialog.attuned
                            onToggled: {
                                dialog.attuned = checked
                                dialog.refreshPreview()
                            }
                        }
                    }

                    Repeater {
                        model: dialog.preview?.derived ?? []
                        QQC2.Label {
                            required property string modelData
                            text: "• " + modelData
                            color: Kirigami.Theme.positiveTextColor
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                        }
                    }
                    Repeater {
                        model: dialog.preview?.warnings ?? []
                        QQC2.Label {
                            required property string modelData
                            text: "• " + modelData
                            color: Kirigami.Theme.neutralTextColor
                            wrapMode: Text.Wrap
                            Layout.fillWidth: true
                        }
                    }

                    Prose {
                        source: {
                            const record = dialog.hit ? Srd.record(dialog.hit.id) : ""
                            return record ? JSON.parse(record).markdown : ""
                        }
                    }
                }
            }
        }

        ProseLine {
            links: "<a href=\"srd:about\">SRD 5.2.1 · CC-BY-4.0</a>"
                   + (dialog.total > dialog.results.length ? " · showing " + dialog.results.length + " of " + dialog.total : "")
            font: Kirigami.Theme.smallFont
            opacity: 0.7
        }
    }
}
