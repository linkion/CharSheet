import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

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
        Layout.columnSpan: 2

        headerActions: [
            Kirigami.Action {
                text: "Add Item"
                icon.name: "list-add"
                onTriggered: {
                    if (App.append("/inventory", {}))
                        itemDialog.edit("/inventory", App.c.inventory.length - 1)
                }
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
                        readonly property int index: modelData.index
                        readonly property string summary: (App.d.item_summaries || [])[index] ?? ""
                        readonly property var actions: (App.d.actions || []).filter(a => a.owner === "item" && a.index === index)
                        readonly property bool equippable: ["weapon", "armor", "shield"].includes(item.kind) || (item.kind !== "ammunition" && item.modifiers.length > 0)
                        readonly property bool countable: ["ammunition", "consumable"].includes(item.kind)
                        Layout.fillWidth: true
                        spacing: 0

                        RowLayout {
                            Layout.fillWidth: true

                            QQC2.CheckBox {
                                visible: entry.equippable
                                checked: entry.item.equipped
                                onToggled: App.set("/inventory/" + entry.index + "/equipped", checked)
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
                                QQC2.Label {
                                    visible: text.length > 0
                                    text: [entry.summary, entry.item.notes].filter(s => s).join("  —  ")
                                    opacity: 0.7
                                    Layout.fillWidth: true
                                    elide: Text.ElideRight
                                }
                            }
                            RowLayout {
                                visible: entry.countable
                                spacing: 0
                                QQC2.ToolButton {
                                    text: "−"
                                    enabled: entry.item.quantity > 0
                                    onClicked: App.set("/inventory/" + entry.index + "/quantity", entry.item.quantity - 1)
                                }
                                QQC2.Label {
                                    text: entry.item.quantity
                                    font.bold: true
                                    horizontalAlignment: Text.AlignHCenter
                                    Layout.minimumWidth: Kirigami.Units.gridUnit * 1.5
                                }
                                QQC2.ToolButton {
                                    text: "+"
                                    onClicked: App.set("/inventory/" + entry.index + "/quantity", entry.item.quantity + 1)
                                }
                            }
                            PipRow {
                                visible: entry.item.charges_max > 0 && entry.item.charges_max <= 20
                                total: Math.min(entry.item.charges_max, 20)
                                used: entry.item.charges_spent
                                onUsedEdited: count => App.set("/inventory/" + entry.index + "/charges_spent", count)
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
                                onToggled: App.set("/inventory/" + entry.index + "/attuned", checked)
                            }
                            Repeater {
                                model: entry.actions
                                ActionButton {
                                    required property var modelData
                                    entry: modelData
                                }
                            }
                            QQC2.ToolButton {
                                icon.name: "document-edit"
                                onClicked: itemDialog.edit("/inventory", entry.index)
                                QQC2.ToolTip.text: "Edit"
                                QQC2.ToolTip.visible: hovered
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

    ItemDialog {
        id: itemDialog
        title: "Item"

        readonly property var isWeapon: item => item.kind === "weapon"
        readonly property var ammoOptions: () => [{ key: "", name: "Standard (not tracked)" }].concat(
            (App.c.inventory || []).filter(i => i.kind === "ammunition").map(i => ({ key: i.name, name: i.name })))

        fields: [
            { key: "name", label: "Name", type: "text" },
            { key: "kind", label: "Kind", type: "choice", options: App.catalog.item_kinds },
            { key: "quantity", label: "Quantity", type: "int", from: 0, to: 9999 },
            { key: "weight", label: "Weight each (lb)", type: "number" },
            { key: "equipped", label: "Equipped", type: "bool" },
            { key: "requires_attunement", label: "Requires attunement", type: "bool" },
            { key: "notes", label: "Notes", type: "area" },

            { label: "Weapon", type: "section", visibleWhen: isWeapon },
            { key: "weapon/damage", label: "Damage dice", type: "text", visibleWhen: isWeapon },
            { key: "weapon/damage_type", label: "Damage type", type: "text", visibleWhen: isWeapon },
            { key: "weapon/magic_bonus", label: "Magic bonus", type: "int", from: -5, to: 10, visibleWhen: isWeapon },
            { key: "weapon/mastery", label: "Mastery", type: "choice", options: App.masteryOptions, visibleWhen: isWeapon },
            { key: "weapon/proficient", label: "Proficient", type: "bool", visibleWhen: isWeapon },
            { key: "weapon/finesse", label: "Finesse", type: "bool", visibleWhen: isWeapon },
            { key: "weapon/light", label: "Light", type: "bool", visibleWhen: isWeapon },
            { key: "weapon/ranged", label: "Ranged weapon", type: "bool", visibleWhen: isWeapon },
            { key: "weapon/thrown", label: "Thrown", type: "bool", visibleWhen: isWeapon },
            { key: "weapon/range", label: "Range", type: "text", placeholder: "e.g. 60/240", visibleWhen: item => isWeapon(item) && (item.weapon?.ranged || item.weapon?.thrown) },
            { key: "weapon/capacity", label: "Shots before reload (0 = none)", type: "int", from: 0, to: 100, visibleWhen: item => isWeapon(item) && item.weapon?.ranged },
            { key: "weapon/ammo", label: "Ammunition", type: "choice", options: ammoOptions, visibleWhen: item => isWeapon(item) && item.weapon?.ranged },

            { label: "Armor", type: "section", visibleWhen: item => item.kind === "armor" || item.kind === "shield" },
            { key: "armor", label: "Armor", type: "choice", options: [{ key: null, name: "None" }].concat(App.catalog.armor), visibleWhen: item => item.kind === "armor" },
            { key: "ac_bonus", label: "Magic AC bonus", type: "int", from: 0, to: 5, visibleWhen: item => item.kind === "armor" || item.kind === "shield" },

            { label: "Charges", type: "section" },
            { key: "charges_max", label: "Charges (0 = none)", type: "int", from: 0, to: 100 },
            { key: "recharge", label: "Recharge", type: "choice", options: App.catalog.recharges, visibleWhen: item => item.charges_max > 0 },

            { label: "Modifiers (while equipped; ammunition: when fired)", type: "section" },
            { key: "modifiers", label: "Modifiers", type: "modifiers" },

            { label: "Actions", type: "section" },
            { key: "actions", label: "Actions", type: "actions", owner: "item" }
        ]
    }
}
