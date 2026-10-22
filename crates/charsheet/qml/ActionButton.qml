import QtQuick
import QtQuick.Controls as QQC2
import io.github.porave.charsheet

// Uses an entry of derived.actions. Spells that need a slot open a menu of the slots (and
// free casts) they can use.
QQC2.Button {
    id: button

    required property var entry
    readonly property bool needsSlot: entry.owner === "spell" && entry.spell_level > 0
    readonly property var options: {
        if (!needsSlot)
            return []
        const out = []
        const free = entry.resource
        if (free && free.max > 0)
            out.push({ label: "Free cast (" + free.left + " of " + free.max + " left)", level: 0, enabled: free.left > 0 })
        const slots = App.d.spell_slots || []
        for (let level = entry.spell_level; level <= 9; level++) {
            let max = slots[level - 1] || 0
            let left = max - (App.c.spell_slots_used[level - 1] || 0)
            if (App.d.pact_slots > 0 && App.d.pact_slot_level === level) {
                max += App.d.pact_slots
                left += App.d.pact_slots - (App.c.pact_slots_used || 0)
            }
            if (max > 0)
                out.push({ label: "Level " + level + " slot (" + left + " of " + max + " left)", level: level, enabled: left > 0 })
        }
        return out
    }

    text: entry.owner === "spell" ? "Cast" : entry.name
    icon.name: entry.owner === "spell" ? "tools-wizard" : "media-playback-start"
    enabled: entry.available
    onClicked: needsSlot ? menu.popup() : App.useAction(entry, 0)
    QQC2.ToolTip.text: entry.roll ? "Rolls " + entry.roll : ""
    QQC2.ToolTip.visible: hovered && entry.roll.length > 0

    QQC2.Menu {
        id: menu

        Instantiator {
            model: button.options
            delegate: QQC2.MenuItem {
                required property var modelData
                text: modelData.label
                enabled: modelData.enabled
                onTriggered: App.useAction(button.entry, modelData.level)
            }
            onObjectAdded: (index, object) => menu.insertItem(index, object)
            onObjectRemoved: (index, object) => menu.removeItem(object)
        }
    }
}
