pragma Singleton
import QtCore
import QtQuick
import io.github.porave.charsheet

// Shared state: the library, the open sheet, its parsed view and edit helpers.
QtObject {
    id: app

    readonly property CharacterLibrary library: CharacterLibrary {}
    readonly property CharacterSheet sheet: CharacterSheet {}

    // Picks up changes made elsewhere.
    function syncNow() {
        library.sync()
        if (sheet.loaded)
            sheet.sync()
    }

    // The MCP server signals its changes as it makes them.
    readonly property LibraryWatcher watcher: LibraryWatcher {
        onLibraryChanged: app.syncNow()
    }

    // Catches what isn't signalled, such as changes from another window.
    readonly property Timer syncTimer: Timer {
        interval: 5000
        repeat: true
        running: true
        onTriggered: app.syncNow()
    }

    // Every new roll, made here or elsewhere.
    readonly property Connections rollWatcher: Connections {
        target: app.sheet
        function onRollsThrown(rollsJson) {
            app.thrown(JSON.parse(rollsJson))
        }
    }

    readonly property Settings appearance: Settings {
        category: "appearance"
        // Throws 3D dice for every roll, rather than only showing a notification.
        property bool dice3d: true
    }

    readonly property var entries: JSON.parse(library.entries)
    readonly property var view: JSON.parse(sheet.view)
    readonly property var c: view.character || ({})
    readonly property var d: view.derived || ({})
    readonly property var rolls: JSON.parse(sheet.rolls)
    readonly property var catalog: JSON.parse(sheet.catalog())

    // "normal", "advantage" or "disadvantage"; applies to every d20 roll.
    property string rollMode: "normal"

    // Tab the next opened sheet starts on.
    property int requestedTab: 0

    // New rolls (StoredRoll objects, oldest first), from any source, including the MCP server.
    signal thrown(var rolls)
    // A short message for a passive notification, e.g. what an action did.
    signal notice(string message)
    // Asks for a record of the rules to be shown.
    signal ruleRequested(string id)
    // Asks for the details of an entry of a list, such as ("/spells", 2), to be shown.
    signal detailsRequested(string listPointer, int index)
    // Asks the tab that owns the list to open its edit dialog for the entry.
    signal editRequested(string listPointer, int index)

    function signed(n) {
        return (n >= 0 ? "+" : "") + n
    }

    function set(pointer, value) {
        return sheet.set(pointer, JSON.stringify(value === undefined ? null : value))
    }

    function append(pointer, item) {
        return sheet.append(pointer, JSON.stringify(item || {}))
    }

    function removeAt(pointer, index) {
        return sheet.removeAt(pointer, index)
    }

    function roll(expression, label, mode) {
        sheet.roll(expression, label || expression, mode || "normal")
    }

    // Opens a link from rendered text: srd: links show the rules, others go to the browser.
    function openLink(link) {
        if (link.startsWith("srd:"))
            ruleRequested(link.slice(4))
        else
            Qt.openUrlExternally(link)
    }

    function escaped(text) {
        return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
    }

    // A term as StyledText, linked to its rules if the SRD has it.
    function ruleLink(name, categories, label) {
        const id = Srd.find(name, JSON.stringify(categories))
        const shown = escaped(label || name)
        return id ? "<a href=\"srd:" + id + "\">" + shown + "</a>" : shown
    }

    function report(message) {
        if (message.length > 0)
            notice(message)
    }

    // The value at a JSON Pointer such as "/inventory/2/modifiers", or undefined.
    function at(pointer) {
        return pointer.split("/").filter(p => p).reduce((o, k) => o == null ? undefined : o[k], c)
    }

    // Rolls something on the sheet, named by a play::RollTarget, so the roll lists its sources.
    function rollCheck(target) {
        sheet.rollCheck(JSON.stringify(target), rollMode)
    }

    function capitalized(text) {
        return text.charAt(0).toUpperCase() + text.slice(1)
    }

    readonly property var masteryOptions: [""].concat(["Cleave", "Graze", "Nick", "Push", "Sap", "Slow", "Topple", "Vex"])
        .map(m => ({ key: m, name: m || "None" }))

    // Uses an entry of derived.actions; slotLevel is the spell slot to cast with, 0 for a free use.
    function useAction(action, slotLevel) {
        let message
        if (action.owner === "item")
            message = sheet.useItemAction(action.index, action.action)
        else if (action.owner === "feature")
            message = sheet.useFeatureAction(action.index, action.action)
        else
            message = sheet.castSpell(action.index, slotLevel || 0)
        report(message)
    }

    function nameOf(list, key) {
        for (const option of list || [])
            if (option.key === key)
                return option.name
        return key
    }
}
