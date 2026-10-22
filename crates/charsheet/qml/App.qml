pragma Singleton
import QtQuick
import io.github.porave.charsheet

// Shared state: the library, the open sheet, its parsed view and edit helpers.
QtObject {
    id: app

    readonly property CharacterLibrary library: CharacterLibrary {}
    readonly property CharacterSheet sheet: CharacterSheet {}

    // Picks up changes made elsewhere, such as by the MCP server.
    readonly property Timer syncTimer: Timer {
        interval: 1000
        repeat: true
        running: true
        onTriggered: {
            app.library.sync()
            if (app.sheet.loaded)
                app.sheet.sync()
        }
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

    signal rolled(var result)
    // A short message for a passive notification, e.g. what an action did.
    signal notice(string message)

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
        const json = sheet.roll(expression, label || expression, mode || "normal")
        if (json.length > 0)
            rolled(JSON.parse(json))
    }

    // Shows a roll the backend made (JSON from CharacterSheet), like roll() does.
    function showRoll(json) {
        if (json.length > 0)
            rolled(JSON.parse(json))
    }

    function report(message) {
        if (message.length > 0)
            notice(message)
    }

    // The value at a JSON Pointer such as "/inventory/2/modifiers", or undefined.
    function at(pointer) {
        return pointer.split("/").filter(p => p).reduce((o, k) => o == null ? undefined : o[k], c)
    }

    function rollD20(modifier, label) {
        roll("1d20" + (modifier !== 0 ? signed(modifier) : ""), label, rollMode)
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
