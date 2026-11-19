import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Everything about one entry of a list (spell, item, feature, attack, effect): a summary line,
// its attacks, the switches you flip in play, its other mechanics, its full text and what it
// lets the character do. Follows the sheet as it changes.
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
    // Positions in derived.weapon_attacks of this item's attacks, when it is an equipped weapon.
    readonly property var weaponAttacks: kind === "inventory" && valid
        ? (App.d.weapon_attacks || []).map((a, i) => a.item === index ? i : -1).filter(i => i >= 0) : []
    readonly property var manualAttack: kind === "attacks" && valid ? (App.d.attacks || [])[index] : undefined
    readonly property bool equippable: kind === "inventory" && valid
        && (["weapon", "armor", "shield"].includes(entry.kind) || (entry.kind !== "ammunition" && (entry.modifiers || []).length > 0))
    readonly property string subtitle: valid ? subtitleOf(kind, entry).filter(s => s).join("  ·  ") : ""
    // The entry's modifiers, one line each; those that switch on and off carry their toggle.
    readonly property var modifierLines: {
        if (!valid)
            return []
        const summaries = kind === "inventory" ? App.d.item_summaries
                        : kind === "features" ? App.d.feature_summaries
                        : kind === "effects" ? App.d.effect_summaries : undefined
        const summary = (summaries || [])[index] ?? ""
        const toggles = (App.d.toggles || []).filter(t => t.pointer.startsWith(entryPointer + "/"))
        return summary.split(" · ").filter(s => s).map(text => ({
            text: text,
            toggle: toggles.find(t => text === t.description || text === t.description + " (off)")
        }))
    }

    // Caption and StyledText value pairs; a pips entry is drawn as a PipRow instead.
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

    function lasts(e) {
        return e.rounds > 0 ? e.rounds + (e.rounds === 1 ? " more turn" : " more turns") : "Until removed"
    }

    // The line under the title: what the entry is, in a few words.
    function subtitleOf(kind, e) {
        switch (kind) {
        case "spells": return [
            e.level === 0 ? "Cantrip" : ordinal(e.level) + " level",
            App.nameOf(App.catalog.activations, e.casting_time),
            e.concentration ? "Concentration" : "",
            e.ritual ? "Ritual" : ""
        ]
        case "inventory": return [
            App.nameOf(App.catalog.item_kinds, e.kind),
            e.quantity !== 1 ? "×" + e.quantity : "",
            e.weight > 0 ? e.weight + " lb" + (e.quantity !== 1 ? " each" : "") : "",
            e.requires_attunement ? "Requires attunement" : ""
        ]
        case "features": return [e.source]
        case "attacks": return [abilityText(e.ability), e.proficient ? "Proficient" : "Not proficient"]
        case "effects": return [e.source, e.concentration ? "Concentration" : "", lasts(e)]
        }
        return []
    }

    function factsOf(kind, e) {
        switch (kind) {
        case "spells": return [
            { label: "Free casts", pips: e.free_uses_max > 0, total: e.free_uses_max, used: e.free_uses_spent, key: "free_uses_spent" },
            { label: "Rolls", value: e.roll ? App.escaped(e.roll + (e.damage_type ? " " + e.damage_type : "")
                                                         + (e.roll_effect !== "none" ? " (" + App.nameOf(App.catalog.roll_effects, e.roll_effect) + ")" : "")) : "" },
            { label: "Effect on you", value: e.effect ? App.escaped(e.effect.name) : "" }
        ]
        case "inventory": {
            const w = e.weapon
            // An equipped weapon's attack card already shows its damage, range and properties.
            const loose = w && weaponAttacks.length === 0
            const properties = loose ? [["Finesse", w.finesse], ["Light", w.light], ["Thrown", w.thrown]]
                .filter(p => p[1]).map(p => App.ruleLink(p[0], ["weapon_property"])) : []
            return [
                { label: "Damage", value: loose ? App.escaped(w.damage + (w.damage_type ? " " + w.damage_type : "") + (w.versatile ? " (" + w.versatile + " two-handed)" : "")) : "" },
                { label: "Range", value: loose ? App.escaped(w.range || (w.ranged ? "" : "Melee")) : "" },
                { label: "Properties", value: properties.join(", ") },
                { label: "Mastery", value: loose && w.mastery ? App.ruleLink(w.mastery, ["mastery_property"]) : "" },
                { label: "Magic bonus", value: w && w.magic_bonus ? App.signed(w.magic_bonus) : e.ac_bonus ? App.signed(e.ac_bonus) + " AC" : "" },
                { label: "Armor", value: e.kind === "armor" && e.armor ? App.escaped(App.nameOf(App.catalog.armor, e.armor)) : "" },
                { label: "Total weight", value: e.weight > 0 && e.quantity > 1 ? (e.weight * e.quantity).toFixed(1) + " lb" : "" },
                { label: "Charges", pips: e.charges_max > 0 && e.charges_max <= 20, total: e.charges_max, used: e.charges_spent, key: "charges_spent" },
                { label: "Charges", value: e.charges_max > 20 ? (e.charges_max - e.charges_spent) + " / " + e.charges_max : "" },
                { label: "Recharge", value: e.charges_max > 0 ? App.escaped(rechargeText(e.recharge) + (e.recharge === "dawn" ? ", regains " + (e.regain || "all") : "")) : "" }
            ]
        }
        case "features": return [
            { label: "Uses", pips: e.uses_max > 0, total: Math.min(e.uses_max, 20), used: e.uses_spent, key: "uses_spent" },
            { label: "Recharge", value: e.uses_max > 0 ? App.escaped(rechargeText(e.recharge)) : "" }
        ]
        case "attacks": return [
            { label: "Damage type", value: App.escaped(e.damage_type ?? "") },
            { label: "Mastery", value: e.mastery ? App.ruleLink(e.mastery, ["mastery_property"]) : "" }
        ]
        }
        return []
    }

    // Deleting the entry, here or from the MCP server, leaves nothing to show.
    onValidChanged: if (!valid && opened) close()

    title: entry.name ?? ""
    standardButtons: Kirigami.Dialog.Close
    // Wrapped text has no width of its own, so the dialog would grow to fit each paragraph
    // on one line: hold it to a reading width that follows the window, and all of a narrow one.
    preferredWidth: Math.min(absoluteMaximumWidth,
        Math.max(Kirigami.Units.gridUnit * 30, Math.min(Kirigami.Units.gridUnit * 40, (parent?.width ?? 0) * 0.55)))
    maximumWidth: preferredWidth
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

    // Reports no width of its own: the dialog's scroll view would otherwise let the content,
    // as wide as its longest unwrapped line, be scrolled sideways. It takes the width the
    // dialog hands it, and the text wraps to that.
    Item {
        implicitWidth: 0
        implicitHeight: column.implicitHeight

        ColumnLayout {
            id: column
            width: parent.width
            spacing: Kirigami.Units.largeSpacing

            QQC2.Label {
                visible: dialog.subtitle.length > 0
                text: dialog.subtitle
                wrapMode: Text.WordWrap
                opacity: 0.7
                Layout.fillWidth: true
            }

            // What it does in a fight, with the rolls a click away.
            Repeater {
                model: dialog.weaponAttacks
                SheetRow {
                    required property int modelData
                    alternate: true
                    WeaponAttackRow {
                        attackIndex: modelData
                        showToggles: false
                    }
                }
            }
            SheetRow {
                visible: !!dialog.manualAttack
                alternate: true
                QQC2.Label {
                    text: "Attack"
                    font.bold: true
                    Layout.fillWidth: true
                }
                RollButton {
                    modifier: dialog.manualAttack ? dialog.manualAttack.to_hit : 0
                    target: ({ kind: "manual_attack", attack: dialog.index })
                    rollLabel: (dialog.entry.name ?? "") + " attack"
                }
                QQC2.Button {
                    text: dialog.manualAttack ? dialog.manualAttack.damage : ""
                    flat: true
                    onClicked: App.rollCheck({ kind: "manual_damage", attack: dialog.index, crit: false })
                    QQC2.ToolTip.text: "Roll damage"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.ToolButton {
                    text: "Crit"
                    onClicked: App.rollCheck({ kind: "manual_damage", attack: dialog.index, crit: true })
                    QQC2.ToolTip.text: "Roll critical damage (dice doubled)"
                    QQC2.ToolTip.visible: hovered
                }
            }

            // State that changes in play, set right here.
            Flow {
                id: state
                readonly property bool attunable: dialog.kind === "inventory" && !!dialog.entry.requires_attunement
                readonly property bool preparable: dialog.kind === "spells" && dialog.entry.level > 0
                visible: dialog.equippable || attunable || preparable
                spacing: Kirigami.Units.largeSpacing
                Layout.fillWidth: true

                QQC2.CheckBox {
                    visible: dialog.equippable
                    text: "Equipped"
                    checked: !!dialog.entry.equipped
                    onToggled: App.set(dialog.entryPointer + "/equipped", checked)
                }
                QQC2.CheckBox {
                    visible: state.attunable
                    text: "Attuned"
                    checked: !!dialog.entry.attuned
                    onToggled: App.set(dialog.entryPointer + "/attuned", checked)
                }
                QQC2.CheckBox {
                    visible: state.preparable
                    text: "Prepared"
                    checked: !!dialog.entry.prepared
                    onToggled: App.set(dialog.entryPointer + "/prepared", checked)
                }
            }

            // The rest of its mechanics, captions above values, filling the width.
            GridLayout {
                visible: dialog.facts.length > 0
                columns: Math.max(1, Math.floor(column.width / (Kirigami.Units.gridUnit * 10)))
                columnSpacing: Kirigami.Units.largeSpacing * 2
                rowSpacing: Kirigami.Units.largeSpacing
                Layout.fillWidth: true

                Repeater {
                    model: dialog.facts

                    ColumnLayout {
                        id: fact
                        required property var modelData
                        spacing: 0
                        Layout.fillWidth: true
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 10
                        Layout.alignment: Qt.AlignTop

                        QQC2.Label {
                            text: fact.modelData.label
                            font: Kirigami.Theme.smallFont
                            opacity: 0.7
                        }
                        QQC2.Label {
                            visible: !fact.modelData.pips
                            text: fact.modelData.value ?? ""
                            textFormat: Text.StyledText
                            wrapMode: Text.WordWrap
                            Layout.fillWidth: true
                            onLinkActivated: link => App.openLink(link)
                            HoverHandler {
                                cursorShape: parent.hoveredLink ? Qt.PointingHandCursor : undefined
                            }
                        }
                        PipRow {
                            visible: !!fact.modelData.pips
                            total: fact.modelData.pips ? fact.modelData.total : 0
                            used: fact.modelData.used ?? 0
                            onUsedEdited: count => App.set(dialog.entryPointer + "/" + fact.modelData.key, count)
                        }
                    }
                }
            }

            ColumnLayout {
                visible: dialog.modifierLines.length > 0
                spacing: Kirigami.Units.smallSpacing
                Layout.fillWidth: true

                Kirigami.Heading {
                    text: dialog.kind === "inventory" ? "While equipped" : dialog.kind === "features" ? "Always on" : "Modifiers"
                    level: 4
                }
                Repeater {
                    model: dialog.modifierLines

                    Loader {
                        id: line
                        required property var modelData
                        Layout.fillWidth: true
                        sourceComponent: modelData.toggle ? switchLine : textLine

                        Component {
                            id: textLine
                            QQC2.Label {
                                text: "•  " + line.modelData.text
                                wrapMode: Text.WordWrap
                                opacity: line.modelData.text.endsWith("(off)") ? 0.6 : 1
                            }
                        }
                        // Modes such as a weapon's Fire mode, saved with the sheet.
                        Component {
                            id: switchLine
                            RowLayout {
                                readonly property var toggle: line.modelData.toggle
                                QQC2.Switch {
                                    text: parent.toggle.label
                                    checked: parent.toggle.enabled
                                    onToggled: App.set(parent.toggle.pointer + "/enabled", checked)
                                }
                                QQC2.Label {
                                    visible: parent.toggle.label !== parent.toggle.description
                                    text: parent.toggle.description
                                    wrapMode: Text.WordWrap
                                    opacity: 0.7
                                    Layout.fillWidth: true
                                }
                            }
                        }
                    }
                }
            }

            ColumnLayout {
                visible: dialog.body.length > 0
                spacing: Kirigami.Units.smallSpacing
                Layout.fillWidth: true

                Kirigami.Heading {
                    text: dialog.kind === "inventory" || dialog.kind === "attacks" ? "Notes" : "Description"
                    level: 4
                }
                Prose {
                    source: dialog.body
                }
            }

            QQC2.Label {
                visible: dialog.facts.length === 0 && dialog.body.length === 0 && dialog.actions.length === 0
                         && dialog.modifierLines.length === 0 && dialog.weaponAttacks.length === 0
                text: "Nothing written down yet. Use Edit to add details."
                opacity: 0.7
            }

            ColumnLayout {
                visible: dialog.actions.length > 0
                spacing: Kirigami.Units.smallSpacing
                Layout.fillWidth: true

                Kirigami.Heading {
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
    }
}
