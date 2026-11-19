import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

SheetTab {
    id: tab

    readonly property var levelNames: ["Cantrips", "1st Level", "2nd Level", "3rd Level", "4th Level", "5th Level", "6th Level", "7th Level", "8th Level", "9th Level"]
    readonly property var indexedSpells: (App.c.spells || []).map((spell, index) => ({ spell, index }))

    Section {
        title: "Spellcasting"

        Repeater {
            model: App.d.casting

            RowLayout {
                required property var modelData
                Layout.fillWidth: true

                QQC2.Label {
                    text: modelData.class + " (" + modelData.ability + ")"
                    Layout.fillWidth: true
                }
                QQC2.Label { text: "Save DC" ; opacity: 0.7 }
                QQC2.Label {
                    text: modelData.save_dc
                    font.bold: true
                }
                QQC2.Label { text: "Attack"; opacity: 0.7 }
                RollButton {
                    modifier: modelData.attack
                    target: ({ kind: "spell_attack", ability: modelData.ability.toLowerCase() })
                    rollLabel: "Spell attack"
                }
            }
        }

        QQC2.Label {
            visible: (App.d.casting || []).length === 0
            text: "None of your classes cast spells. Third casters (Eldritch Knight, Arcane Trickster) can be turned on in the Build tab."
            wrapMode: Text.WordWrap
            opacity: 0.7
            Layout.fillWidth: true
        }
    }

    Section {
        title: "Spell Slots"

        Repeater {
            model: App.d.spell_slots

            RowLayout {
                required property int modelData
                required property int index
                visible: modelData > 0
                Layout.fillWidth: true

                QQC2.Label {
                    text: tab.levelNames[index + 1]
                    font.family: Style.labelFamily
                    font.capitalization: Style.decorated ? Font.AllUppercase : Font.MixedCase
                    font.letterSpacing: Style.decorated ? 1 : 0
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 5
                }
                PipRow {
                    total: modelData
                    used: App.c.spell_slots_used[index]
                    onUsedEdited: count => App.set("/spell_slots_used/" + index, count)
                }
                Item { Layout.fillWidth: true }
            }
        }

        RowLayout {
            visible: (App.d.pact_slots ?? 0) > 0
            Layout.fillWidth: true
            QQC2.Label {
                text: "Pact (level " + App.d.pact_slot_level + ")"
                font.family: Style.labelFamily
                font.capitalization: Style.decorated ? Font.AllUppercase : Font.MixedCase
                font.letterSpacing: Style.decorated ? 1 : 0
                Layout.preferredWidth: Kirigami.Units.gridUnit * 5
            }
            PipRow {
                total: App.d.pact_slots ?? 0
                used: App.c.pact_slots_used ?? 0
                onUsedEdited: count => App.set("/pact_slots_used", count)
            }
            Item { Layout.fillWidth: true }
        }

        QQC2.Label {
            visible: (App.d.spell_slots || []).every(n => n === 0) && !(App.d.pact_slots > 0)
            text: "No spell slots."
            opacity: 0.7
        }
    }

    Section {
        title: "Spells"
        Layout.columnSpan: tab.columns

        headerActions: [
            Kirigami.Action {
                text: "From Compendium"
                icon.name: "search"
                onTriggered: compendium.show()
            },
            Kirigami.Action {
                text: "Add Spell"
                icon.name: "list-add"
                onTriggered: {
                    if (App.append("/spells", {}))
                        spellDialog.edit("/spells", App.c.spells.length - 1)
                }
            }
        ]

        Repeater {
            model: 10

            ColumnLayout {
                id: levelGroup
                required property int index
                readonly property var spells: tab.indexedSpells.filter(e => e.spell.level === index)
                visible: spells.length > 0
                Layout.fillWidth: true

                Kirigami.Heading {
                    text: tab.levelNames[levelGroup.index]
                    level: 4
                }

                Repeater {
                    model: levelGroup.spells

                    SheetRow {
                        required property var modelData
                        required property int index
                        alternate: index % 2 === 1
                        clickable: true
                        onClicked: App.detailsRequested("/spells", modelData.index)

                        QQC2.CheckBox {
                            visible: levelGroup.index > 0
                            checked: modelData.spell.prepared
                            onToggled: App.set("/spells/" + modelData.index + "/prepared", checked)
                            QQC2.ToolTip.text: "Prepared"
                            QQC2.ToolTip.visible: hovered
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0
                            QQC2.Label {
                                text: App.nameOf(App.catalog.activations, modelData.spell.casting_time) + "  ·  " + modelData.spell.name
                                      + (modelData.spell.concentration ? "  [C]" : "")
                                      + (modelData.spell.ritual ? "  [R]" : "")
                                font.bold: modelData.spell.prepared || levelGroup.index === 0
                                Layout.fillWidth: true
                                elide: Text.ElideRight
                            }
                            ProseLine {
                                source: modelData.spell.notes
                                opacity: 0.7
                            }
                        }
                        RuleButton {
                            name: modelData.spell.name
                        }
                        PipRow {
                            visible: modelData.spell.free_uses_max > 0
                            total: Math.min(modelData.spell.free_uses_max, 10)
                            used: modelData.spell.free_uses_spent
                            onUsedEdited: count => App.set("/spells/" + modelData.index + "/free_uses_spent", count)
                            QQC2.ToolTip.text: "Free casts"
                            QQC2.ToolTip.visible: freeHover.hovered
                            HoverHandler { id: freeHover }
                        }
                        Repeater {
                            model: (App.d.actions || []).filter(a => a.owner === "spell" && a.index === modelData.index)
                            ActionButton {
                                required property var modelData
                                entry: modelData
                            }
                        }
                    }
                }
            }
        }

        QQC2.Label {
            visible: tab.indexedSpells.length === 0
            text: "No spells yet."
            opacity: 0.7
        }
    }

    Connections {
        target: App
        function onEditRequested(listPointer, index) {
            if (listPointer === "/spells")
                spellDialog.edit(listPointer, index)
        }
    }
    CompendiumDialog {
        id: compendium
        title: "Add a Spell"
        categories: [{ key: "spell", name: "Spells" }]
    }
    ItemDialog {
        id: spellDialog
        title: "Spell"
        fields: [
            { key: "name", label: "Name", type: "text" },
            { key: "level", label: "Level (0 = cantrip)", type: "int", from: 0, to: 9 },
            { key: "prepared", label: "Prepared", type: "bool" },
            { key: "concentration", label: "Concentration", type: "bool" },
            { key: "ritual", label: "Ritual", type: "bool" },
            { key: "casting_time", label: "Casting time", type: "choice", options: App.catalog.activations },
            { key: "notes", label: "Notes", type: "area" },
            { label: "Casting", type: "section" },
            { key: "free_uses_max", label: "Free casts per long rest", type: "int", from: 0, to: 20 },
            { key: "roll", label: "Roll when cast", type: "text", placeholder: "e.g. 2d8+3", visibleWhen: spell => !spell.roll_formula },
            { key: "roll_formula", label: "Roll formula", type: "text", placeholder: "e.g. {cantrip}d10 or 2d8+{spell}" },
            { key: "roll_effect", label: "With the roll", type: "choice", options: App.catalog.roll_effects, visibleWhen: spell => (spell.roll || "").length > 0 },
            { key: "damage_type", label: "Damage type", type: "text", visibleWhen: spell => (spell.roll || "").length > 0 },
            { label: "Effect on you", type: "section" },
            { key: "effect", label: "Effect", type: "effect" }
        ]
    }
}
