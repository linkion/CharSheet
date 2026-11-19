import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Everything about one entry of a list (spell, item, feature, attack, effect): its mechanics,
// its full text and what it lets the character do. Follows the sheet as it changes.
Kirigami.Dialog {
    id: dialog

    property string listPointer
    property int index: -1
    // "spells", "inventory", "features", "attacks" or "effects".
    readonly property string kind: listPointer.slice(1)
    readonly property var list: App.view.character ? App.at(listPointer) : undefined
    readonly property bool valid: !!list && index >= 0 && index < list.length
    readonly property var entry: valid ? list[index] : ({})
    readonly property string entryPointer: listPointer + "/" + index
    readonly property string owner: ({ spells: "spell", inventory: "item", features: "feature" })[kind] ?? ""
    readonly property var actions: owner ? (App.d.actions || []).filter(a => a.owner === owner && a.index === index) : []
    readonly property string rule: valid ? Srd.find(entry.name ?? "", JSON.stringify(kind === "spells" ? ["spell"] : [])) : ""
    readonly property string body: (kind === "features" ? entry.description : entry.notes) ?? ""

    // Label and StyledText value pairs; a pips entry is drawn as a PipRow instead.
    // Reads the list itself rather than `entry`, which may not have caught up with a new
    // listPointer or index when this is evaluated.
    readonly property var facts: {
        const l = App.view.character ? App.at(listPointer) : undefined
        const e = l && index >= 0 && index < l.length ? l[index] : undefined
        return e ? factsOf(listPointer.slice(1), e).filter(f => f.pips || (f.value !== undefined && f.value !== "")) : []
    }

    function show(pointer, i) {
        listPointer = pointer
        index = i
        open()
    }

    function yes(flag) {
        return flag ? "Yes" : ""
    }

    function ordinal(n) {
        return n + (n === 1 ? "st" : n === 2 ? "nd" : n === 3 ? "rd" : "th")
    }

    function rechargeText(recharge) {
        return recharge === "none" ? "" : App.nameOf(App.catalog.recharges, recharge)
    }

    function abilityText(key) {
        return App.nameOf(App.catalog.abilities, key)
    }

    function factsOf(kind, e) {
        const i = dialog.index
        switch (kind) {
        case "spells": return [
            { label: "Level", value: e.level === 0 ? "Cantrip" : ordinal(e.level) + " level" },
            { label: "Casting time", value: App.escaped(App.nameOf(App.catalog.activations, e.casting_time)) },
            { label: "Concentration", value: yes(e.concentration) },
            { label: "Ritual", value: yes(e.ritual) },
            { label: "Prepared", value: e.level > 0 ? (e.prepared ? "Yes" : "No") : "" },
            { label: "Free casts", pips: e.free_uses_max > 0, total: e.free_uses_max, used: e.free_uses_spent, key: "free_uses_spent" },
            { label: "Rolls", value: e.roll ? App.escaped(e.roll + (e.damage_type ? " " + e.damage_type : "")
                                                         + (e.roll_effect !== "none" ? " (" + App.nameOf(App.catalog.roll_effects, e.roll_effect) + ")" : "")) : "" },
            { label: "Effect on you", value: e.effect ? App.escaped(e.effect.name) : "" }
        ]
        case "inventory": {
            const attacks = (App.d.weapon_attacks || []).filter(a => a.item === i)
            const w = e.weapon
            return [
                { label: "Kind", value: App.escaped(App.nameOf(App.catalog.item_kinds, e.kind)) },
                { label: "Quantity", value: e.quantity !== 1 ? String(e.quantity) : "" },
                { label: "Weight", value: e.weight > 0 ? e.weight + " lb" + (e.quantity !== 1 ? " each, " + (e.weight * e.quantity).toFixed(1) + " lb in all" : "") : "" },
                { label: "Equipped", value: ["weapon", "armor", "shield"].includes(e.kind) || e.modifiers.length > 0 ? (e.equipped ? "Yes" : "No") : "" },
                { label: "Attunement", value: e.requires_attunement ? (e.attuned ? "Attuned" : "Required, not attuned") : "" },
                { label: "Damage", value: w ? App.escaped(w.damage + (w.damage_type ? " " + w.damage_type : "") + (w.versatile ? " (" + w.versatile + " two-handed)" : "")) : "" },
                { label: "Range", value: w && w.range ? App.escaped(w.range) : "" },
                { label: "Properties", value: attacks.length > 0 ? (attacks[0].properties || []).map(p => App.ruleLink(p.split(" (")[0], ["weapon_property"], p)).join(", ") : "" },
                { label: "Mastery", value: w && w.mastery ? App.ruleLink(w.mastery, ["mastery_property"]) : "" },
                { label: "Magic bonus", value: w && w.magic_bonus ? App.signed(w.magic_bonus) : e.ac_bonus ? App.signed(e.ac_bonus) + " AC" : "" },
                { label: "Attack", value: attacks.map(a => App.escaped((attacks.length > 1 ? a.name + ": " : "") + App.signed(a.to_hit) + " to hit, " + a.damage)).join("<br>") },
                { label: "Armor", value: e.kind === "armor" && e.armor ? App.escaped(App.nameOf(App.catalog.armor, e.armor)) : "" },
                { label: "Charges", pips: e.charges_max > 0 && e.charges_max <= 20, total: e.charges_max, used: e.charges_spent, key: "charges_spent" },
                { label: "Charges", value: e.charges_max > 20 ? (e.charges_max - e.charges_spent) + " / " + e.charges_max : "" },
                { label: "Recharge", value: e.charges_max > 0 ? App.escaped(rechargeText(e.recharge) + (e.recharge === "dawn" ? ", regains " + (e.regain || "all") : "")) : "" },
                { label: "Modifiers", value: App.escaped((App.d.item_summaries || [])[i] ?? "") }
            ]
        }
        case "features": return [
            { label: "Source", value: App.escaped(e.source) },
            { label: "Uses", pips: e.uses_max > 0, total: Math.min(e.uses_max, 20), used: e.uses_spent, key: "uses_spent" },
            { label: "Recharge", value: e.uses_max > 0 ? App.escaped(rechargeText(e.recharge)) : "" },
            { label: "Modifiers", value: App.escaped((App.d.feature_summaries || [])[i] ?? "") }
        ]
        case "attacks": {
            const derived = (App.d.attacks || [])[i]
            return [
                { label: "Ability", value: App.escaped(abilityText(e.ability)) },
                { label: "Proficient", value: e.proficient ? "Yes" : "No" },
                { label: "To hit", value: derived ? App.signed(derived.to_hit) : "" },
                { label: "Damage", value: derived ? App.escaped(derived.damage + (e.damage_type ? " " + e.damage_type : "")) : "" },
                { label: "Mastery", value: e.mastery ? App.ruleLink(e.mastery, ["mastery_property"]) : "" }
            ]
        }
        case "effects": return [
            { label: "Source", value: App.escaped(e.source) },
            { label: "Concentration", value: yes(e.concentration) },
            { label: "Lasts", value: e.rounds > 0 ? e.rounds + (e.rounds === 1 ? " more turn" : " more turns") : "Until removed" },
            { label: "Modifiers", value: App.escaped((App.d.effect_summaries || [])[i] ?? "") }
        ]
        }
        return []
    }

    // Deleting the entry, here or from the MCP server, leaves nothing to show.
    onValidChanged: if (!valid && opened) close()

    title: entry.name ?? ""
    standardButtons: Kirigami.Dialog.Close
    preferredWidth: Kirigami.Units.gridUnit * 32
    padding: Kirigami.Units.largeSpacing

    customFooterActions: [
        Kirigami.Action {
            text: "Rules"
            icon.name: "help-about"
            visible: dialog.rule.length > 0
            onTriggered: App.ruleRequested(dialog.rule)
        },
        Kirigami.Action {
            text: "End Effect"
            icon.name: "dialog-close"
            visible: dialog.kind === "effects"
            onTriggered: App.sheet.endEffect(dialog.index)
        },
        Kirigami.Action {
            text: "Edit"
            icon.name: "document-edit"
            onTriggered: {
                const pointer = dialog.listPointer
                const i = dialog.index
                dialog.close()
                App.editRequested(pointer, i)
            }
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.FormLayout {
            visible: dialog.facts.length > 0
            Layout.fillWidth: true

            Repeater {
                model: dialog.facts

                Loader {
                    id: fact
                    required property var modelData
                    Kirigami.FormData.label: modelData.label + ":"
                    sourceComponent: modelData.pips ? pipsFact : textFact

                    Component {
                        id: textFact
                        QQC2.Label {
                            text: fact.modelData.value
                            textFormat: Text.StyledText
                            wrapMode: Text.WordWrap
                            onLinkActivated: link => App.openLink(link)
                            HoverHandler {
                                cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : undefined
                            }
                        }
                    }
                    Component {
                        id: pipsFact
                        PipRow {
                            total: fact.modelData.total
                            used: fact.modelData.used
                            onUsedEdited: count => App.set(dialog.entryPointer + "/" + fact.modelData.key, count)
                        }
                    }
                }
            }
        }

        Kirigami.Separator {
            visible: dialog.facts.length > 0 && dialog.body.length > 0
            Layout.fillWidth: true
        }

        Prose {
            visible: dialog.body.length > 0
            source: dialog.body
        }

        QQC2.Label {
            visible: dialog.facts.length === 0 && dialog.body.length === 0 && dialog.actions.length === 0
            text: "Nothing written down yet. Use Edit to add details."
            opacity: 0.7
        }

        Kirigami.Heading {
            visible: dialog.actions.length > 0
            text: dialog.owner === "spell" ? "Cast" : "Actions"
            level: 4
        }

        Repeater {
            model: dialog.actions

            SheetRow {
                id: actionRow
                required property var modelData
                required property int index
                alternate: index % 2 === 1

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        text: actionRow.modelData.name + "  ·  " + App.nameOf(App.catalog.activations, actionRow.modelData.activation)
                              + (actionRow.modelData.roll ? "  ·  Rolls " + actionRow.modelData.roll : "")
                        font.bold: true
                        wrapMode: Text.WordWrap
                        Layout.fillWidth: true
                    }
                    Prose {
                        visible: dialog.owner !== "spell" && source.length > 0
                        source: actionRow.modelData.description ?? ""
                        opacity: 0.85
                    }
                }
                QQC2.Label {
                    visible: !!actionRow.modelData.resource
                    text: actionRow.modelData.resource ? actionRow.modelData.resource.left + (actionRow.modelData.resource.label === "left" ? " left" : " / " + actionRow.modelData.resource.max) : ""
                    font.bold: true
                    color: actionRow.modelData.available ? Kirigami.Theme.textColor : Kirigami.Theme.negativeTextColor
                }
                ActionButton {
                    entry: actionRow.modelData
                }
            }
        }
    }
}
