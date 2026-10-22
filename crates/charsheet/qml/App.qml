pragma Singleton
import QtQuick
import io.github.porave.charsheet

// Shared state: the library, the open sheet, its parsed view and edit helpers.
QtObject {
    id: app

    readonly property CharacterLibrary library: CharacterLibrary {}
    readonly property CharacterSheet sheet: CharacterSheet {}

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

    function rollD20(modifier, label) {
        roll("1d20" + (modifier !== 0 ? signed(modifier) : ""), label, rollMode)
    }

    function nameOf(list, key) {
        for (const option of list || [])
            if (option.key === key)
                return option.name
        return key
    }
}
