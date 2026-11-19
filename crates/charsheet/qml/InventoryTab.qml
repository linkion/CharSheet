import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

SheetTab {
    id: tab

    readonly property var groups: [
        { title: "Weapons & Armor", kinds: ["weapon", "armor", "shield"] },
        { title: "Ammunition & Consumables", kinds: ["ammunition", "consumable"] },
        { title: "Gear", kinds: ["gear"] }
    ]
    readonly property var indexedItems: (App.c.inventory || []).map((item, index) => ({ item, index }))

    Section {
        title: "Currency"

        // One small box per coin, copper to platinum, as on the sheet.
        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            Repeater {
                model: [["cp", "Copper"], ["sp", "Silver"], ["ep", "Electrum"], ["gp", "Gold"], ["pp", "Platinum"]]
                delegate: SheetCard {
                    required property var modelData
                    knob: "none"
                    curl: 8
                    contentItem: ColumnLayout {
                        spacing: Kirigami.Units.smallSpacing
                        FrameTitle {
                            text: Style.decorated ? modelData[0] : modelData[1]
                            scale: 1
                            QQC2.ToolTip.text: modelData[1]
                            QQC2.ToolTip.visible: coinHover.hovered
                            HoverHandler { id: coinHover }
                        }
                        SheetSpin {
                            pointer: "/currency/" + modelData[0]
                            fieldValue: App.c.currency ? App.c.currency[modelData[0]] : 0
                            to: 9999999
                        }
                    }
                }
            }
        }
    }

    Section {
        title: "Carrying"

        QQC2.Label {
            text: "Carried: " + (App.d.carried_weight ?? 0).toFixed(1) + " lb of " + (App.d.carrying_capacity ?? 0) + " lb capacity"
        }
        QQC2.ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: App.d.carrying_capacity || 1
            value: Math.min(App.d.carried_weight ?? 0, App.d.carrying_capacity ?? 0)
        }
        QQC2.Label {
            text: "Attuned items: " + (App.d.attuned ?? 0) + " / 3"
            color: (App.d.attuned ?? 0) > 3 ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
        }
    }

    Section {
        title: "Equipment"
        Layout.columnSpan: tab.columns

        headerActions: [
            Kirigami.Action {
                text: "From Compendium"
                icon.name: "search"
                onTriggered: compendium.show()
            },
            Kirigami.Action {
                text: "Add Item"
                icon.name: "list-add"
                onTriggered: itemDialog.create("/inventory")
            }
        ]

        Repeater {
            model: tab.groups

            ColumnLayout {
                id: group
                required property var modelData
                readonly property var entries: tab.indexedItems.filter(e => modelData.kinds.includes(e.item.kind))
                visible: entries.length > 0
                Layout.fillWidth: true

                Kirigami.Heading {
                    text: group.modelData.title
                    level: 4
                }

                Repeater {
                    model: group.entries

                    ColumnLayout {
                        id: entry
                        required property var modelData
                        readonly property var item: modelData.item
                        required property int index
                        readonly property int itemIndex: modelData.index
                        readonly property string summary: (App.d.item_summaries || [])[itemIndex] ?? ""
                        readonly property var actions: (App.d.actions || []).filter(a => a.owner === "item" && a.index === itemIndex)
                        readonly property bool equippable: ["weapon", "armor", "shield"].includes(item.kind) || (item.kind !== "ammunition" && item.modifiers.length > 0)
                        readonly property bool countable: ["ammunition", "consumable"].includes(item.kind)
                        Layout.fillWidth: true
                        spacing: 0

                        SheetRow {
                            alternate: entry.index % 2 === 1
                            clickable: true
                            onClicked: App.detailsRequested("/inventory", entry.itemIndex)

                            QQC2.CheckBox {
                                visible: entry.equippable
                                checked: entry.item.equipped
                                onToggled: App.set("/inventory/" + entry.itemIndex + "/equipped", checked)
                                QQC2.ToolTip.text: "Equipped"
                                QQC2.ToolTip.visible: hovered
                            }
                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0
                                QQC2.Label {
                                    text: entry.item.name + (entry.item.quantity !== 1 && !entry.countable ? "  ×" + entry.item.quantity : "")
                                    font.bold: true
                                    opacity: entry.item.quantity === 0 ? 0.5 : 1
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                                ProseLine {
                                    prefix: entry.summary ?? ""
                                    source: entry.item.notes ?? ""
                                    opacity: 0.7
                                }
                            }
                            RowLayout {
                                visible: entry.countable
                                spacing: 0
                                QQC2.ToolButton {
                                    text: "−"
                                    enabled: entry.item.quantity > 0
                                    onClicked: App.set("/inventory/" + entry.itemIndex + "/quantity", entry.item.quantity - 1)
                                }
                                QQC2.Label {
                                    text: entry.item.quantity
                                    font.bold: true
                                    horizontalAlignment: Text.AlignHCenter
                                    Layout.minimumWidth: Kirigami.Units.gridUnit * 1.5
                                }
                                QQC2.ToolButton {
                                    text: "+"
                                    onClicked: App.set("/inventory/" + entry.itemIndex + "/quantity", entry.item.quantity + 1)
                                }
                            }
                            PipRow {
                                visible: entry.item.charges_max > 0 && entry.item.charges_max <= 20
                                total: Math.min(entry.item.charges_max, 20)
                                used: entry.item.charges_spent
                                onUsedEdited: count => App.set("/inventory/" + entry.itemIndex + "/charges_spent", count)
                            }
                            QQC2.Label {
                                visible: entry.item.charges_max > 20
                                text: (entry.item.charges_max - entry.item.charges_spent) + " / " + entry.item.charges_max + " charges"
                            }
                            QQC2.Label {
                                visible: entry.item.weight > 0
                                text: (entry.item.weight * entry.item.quantity).toFixed(1) + " lb"
                                opacity: 0.7
                            }
                            QQC2.CheckBox {
                                visible: entry.item.requires_attunement
                                text: "Attuned"
                                checked: entry.item.attuned
                                onToggled: App.set("/inventory/" + entry.itemIndex + "/attuned", checked)
                            }
                            Repeater {
                                model: entry.actions
                                ActionButton {
                                    required property var modelData
                                    entry: modelData
                                }
                            }
                        }
                    }
                }
            }
        }

        QQC2.Label {
            visible: (App.c.inventory || []).length === 0
            text: "Nothing carried yet."
            opacity: 0.7
        }
    }

    Connections {
        target: App
        function onEditRequested(listPointer, index) {
            if (listPointer === "/inventory")
                itemDialog.edit(listPointer, index)
        }
    }
    CompendiumDialog {
        id: compendium
        title: "Add an Item"
        categories: [
            { key: "magic_item", name: "Magic items" },
            { key: "weapon", name: "Weapons" },
            { key: "armor", name: "Armor" },
            { key: "gear", name: "Gear" },
            { key: "tool", name: "Tools" }
        ]
    }
    ItemDialog {
        id: itemDialog
        noun: "Item"

        readonly property var isWeapon: item => item.kind === "weapon"
        readonly property var ammoOptions: () => [{ key: "", name: "Standard (not tracked)" }].concat(
            (App.c.inventory || []).filter(i => i.kind === "ammunition").map(i => ({ key: i.name, name: i.name })))

        fields: [
            { key: "name", label: "Name", type: "text", placeholder: "e.g. Longsword" },
            { key: "kind", label: "Kind", type: "choice", options: App.catalog.item_kinds },
            { type: "row", fields: [
                { key: "quantity", label: "Quantity", type: "int", from: 0, to: 9999 },
                { key: "weight", label: "Weight each (lb)", type: "number" }
            ] },
            { label: "", type: "flags", flags: [
                { key: "equipped", label: "Equipped" },
                { key: "requires_attunement", label: "Requires attunement" },
                { key: "attuned", label: "Attuned", visibleWhen: item => item.requires_attunement }
            ] },

            { label: "Weapon", type: "section", visibleWhen: isWeapon },
            { type: "row", fields: [
                { key: "weapon/damage", label: "Damage", type: "text", placeholder: "1d8" },
                { key: "weapon/damage_type", label: "Type", type: "text", placeholder: "slashing" }
            ] },
            { type: "row", fields: [
                { key: "weapon/magic_bonus", label: "Magic bonus", type: "int", from: -5, to: 10 },
                { key: "weapon/mastery", label: "Mastery", type: "choice", options: App.masteryOptions }
            ] },
            { label: "Properties", type: "flags", flags: [
                { key: "weapon/proficient", label: "Proficient", tip: "Add your proficiency bonus to hit" },
                { key: "weapon/finesse", label: "Finesse" },
                { key: "weapon/light", label: "Light" },
                { key: "weapon/ranged", label: "Ranged" },
                { key: "weapon/thrown", label: "Thrown" }
            ] },
            { key: "weapon/range", label: "Range", type: "text", placeholder: "e.g. 60/240", visibleWhen: item => item.weapon?.ranged || item.weapon?.thrown },
            { type: "row", visibleWhen: item => item.weapon?.ranged, fields: [
                { key: "weapon/capacity", label: "Shots before reload", type: "int", from: 0, to: 100 },
                { key: "weapon/ammo", label: "Ammunition", type: "choice", options: ammoOptions }
            ] },

            { label: "Armor", type: "section", visibleWhen: item => item.kind === "armor" || item.kind === "shield" },
            { key: "armor", label: "Armor", type: "choice", options: [{ key: null, name: "None" }].concat(App.catalog.armor), visibleWhen: item => item.kind === "armor" },
            { key: "ac_bonus", label: "Magic AC bonus", type: "int", from: 0, to: 5 },

            { label: "Notes", type: "section" },
            { key: "notes", label: "", type: "area", placeholder: "Description, history, anything worth remembering. Markdown supported." },

            { label: "Charges", type: "section", collapsedWhen: item => !(item.charges_max > 0),
              badge: item => item.charges_max > 0 ? item.charges_max + " max" : "" },
            { type: "row", fields: [
                { key: "charges_max", label: "Charges (0 = none)", type: "int", from: 0, to: 100 },
                { key: "recharge", label: "Recharge", type: "choice", options: App.catalog.recharges, visibleWhen: item => item.charges_max > 0 }
            ] },
            { key: "regain", label: "Regains", type: "text", placeholder: "e.g. 1d6+1; empty regains all", visibleWhen: item => item.charges_max > 0 && item.recharge === "dawn" },

            { label: "Modifiers", type: "section", collapsedWhen: item => !(item.modifiers || []).length, badge: count("modifiers") },
            { label: "Active while equipped; ammunition applies them when fired.", type: "hint" },
            { key: "modifiers", label: "Modifiers", type: "modifiers" },

            { label: "Actions", type: "section", collapsedWhen: item => !(item.actions || []).length, badge: count("actions") },
            { key: "actions", label: "Actions", type: "actions", owner: "item" }
        ]
    }
}
