import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

SheetTab {
    id: tab

    readonly property var masteryOptions: [""].concat(["Cleave", "Graze", "Nick", "Push", "Sap", "Slow", "Topple", "Vex"])
        .map(m => ({ key: m, name: m || "None" }))

    function doubleDice(expression) {
        return expression.replace(/(\d*)d(\d+)/gi, (m, n, sides) => (2 * (parseInt(n) || 1)) + "d" + sides)
    }

    GridLayout {
        Layout.fillWidth: true
        Layout.columnSpan: 2
        columns: 4
        columnSpacing: Kirigami.Units.largeSpacing

        StatTile { label: "Armor Class"; value: App.d.armor_class ?? "" }
        StatTile { label: "Initiative"; rollable: true; rollModifier: App.d.initiative ?? 0 }
        StatTile { label: "Speed"; value: (App.d.speed ?? 0) + " ft" }
        StatTile { label: "Proficiency"; value: App.signed(App.d.proficiency_bonus ?? 0) }
    }

    Section {
        title: "Hit Points"

        RowLayout {
            Layout.fillWidth: true
            Kirigami.Heading {
                text: (App.c.hp?.current ?? 0) + " / " + (App.c.hp?.max ?? 0)
                level: 1
                color: (App.c.hp?.current ?? 1) === 0 ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
            }
            QQC2.Label {
                visible: (App.c.hp?.temp ?? 0) > 0
                text: "+" + (App.c.hp?.temp ?? 0) + " temp"
                color: Kirigami.Theme.positiveTextColor
                font.bold: true
            }
            Item { Layout.fillWidth: true }
        }

        QQC2.ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: App.c.hp?.max ?? 1
            value: App.c.hp?.current ?? 0
        }

        RowLayout {
            Layout.fillWidth: true
            QQC2.SpinBox {
                id: amount
                from: 0
                to: 999
                value: 1
                editable: true
            }
            QQC2.Button {
                text: "Damage"
                icon.name: "list-remove"
                onClicked: App.sheet.damage(amount.value)
            }
            QQC2.Button {
                text: "Heal"
                icon.name: "list-add"
                onClicked: App.sheet.heal(amount.value)
            }
            QQC2.Button {
                text: "Set Temp"
                onClicked: App.sheet.setTempHp(amount.value)
            }
        }

        Kirigami.Separator { Layout.fillWidth: true }

        GridLayout {
            columns: 3
            QQC2.Label { text: "Death save successes" }
            PipRow {
                total: 3
                used: App.c.death_saves?.successes ?? 0
                onUsedEdited: count => App.set("/death_saves/successes", count)
            }
            Item { Layout.fillWidth: true }
            QQC2.Label { text: "Death save failures" }
            PipRow {
                total: 3
                used: App.c.death_saves?.failures ?? 0
                onUsedEdited: count => App.set("/death_saves/failures", count)
            }
            QQC2.Button {
                text: "Roll Death Save"
                icon.name: "roll"
                onClicked: App.rollD20(App.d.d20_penalty, "Death save")
            }
        }
    }

    Section {
        title: "Hit Dice & Rests"

        Repeater {
            model: App.d.hit_dice

            RowLayout {
                required property var modelData
                required property int index
                Layout.fillWidth: true

                QQC2.Label {
                    text: modelData.class + " (d" + modelData.die + ")"
                    Layout.fillWidth: true
                }
                QQC2.Label {
                    text: modelData.remaining + " / " + modelData.total
                    font.bold: true
                }
                QQC2.Button {
                    text: "Spend"
                    enabled: modelData.remaining > 0
                    onClicked: {
                        const message = App.sheet.spendHitDie(index)
                        if (message.length > 0)
                            applicationWindow().showPassiveNotification(message)
                    }
                }
            }
        }

        RowLayout {
            QQC2.Button {
                text: "Short Rest"
                icon.name: "chronometer-pause"
                onClicked: {
                    App.sheet.shortRest()
                    applicationWindow().showPassiveNotification("Pact slots and short-rest features restored")
                }
            }
            QQC2.Button {
                text: "Long Rest"
                icon.name: "go-sleep"
                onClicked: longRestDialog.open()
            }
        }
    }

    Section {
        title: "Attacks"
        Layout.columnSpan: 2

        headerActions: [
            Kirigami.Action {
                text: "Add Attack"
                icon.name: "list-add"
                onTriggered: {
                    if (App.append("/attacks", {}))
                        attackDialog.edit("/attacks", App.c.attacks.length - 1)
                }
            }
        ]

        Repeater {
            model: App.c.attacks

            RowLayout {
                required property var modelData
                required property int index
                readonly property var derived: App.d.attacks[index]
                Layout.fillWidth: true

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        text: modelData.name
                        font.bold: true
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                    QQC2.Label {
                        text: [modelData.damage_type, modelData.mastery ? "Mastery: " + modelData.mastery : "", modelData.notes]
                              .filter(s => s).join(" · ")
                        visible: text.length > 0
                        opacity: 0.7
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                }
                RollButton {
                    modifier: derived ? derived.to_hit : 0
                    rollLabel: modelData.name + " attack"
                }
                QQC2.Button {
                    text: derived ? derived.damage : ""
                    flat: true
                    onClicked: App.roll(derived.damage, modelData.name + " damage")
                    QQC2.ToolTip.text: "Roll damage"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.ToolButton {
                    text: "Crit"
                    onClicked: App.roll(tab.doubleDice(derived.damage), modelData.name + " critical damage")
                    QQC2.ToolTip.text: "Roll critical damage (dice doubled)"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.ToolButton {
                    icon.name: "document-edit"
                    onClicked: attackDialog.edit("/attacks", index)
                    QQC2.ToolTip.text: "Edit"
                    QQC2.ToolTip.visible: hovered
                }
            }
        }

        QQC2.Label {
            visible: (App.c.attacks || []).length === 0
            text: "No attacks yet."
            opacity: 0.7
        }
    }

    Section {
        title: "Conditions"
        Layout.columnSpan: 2

        RowLayout {
            QQC2.Label { text: "Exhaustion" }
            SheetSpin {
                pointer: "/exhaustion"
                fieldValue: App.c.exhaustion ?? 0
                to: 6
            }
            QQC2.Label {
                visible: (App.c.exhaustion ?? 0) > 0
                text: (App.c.exhaustion >= 6 ? "Dead. " : "") + App.d.d20_penalty + " to d20 tests, −" + (App.c.exhaustion * 5) + " ft speed"
                opacity: 0.8
            }
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            Repeater {
                model: App.catalog.conditions
                Kirigami.Chip {
                    required property var modelData
                    text: modelData.name
                    checkable: true
                    closable: false
                    checked: (App.c.conditions || []).includes(modelData.key)
                    onToggled: {
                        const list = (App.c.conditions || []).filter(k => k !== modelData.key)
                        if (checked)
                            list.push(modelData.key)
                        App.set("/conditions", list)
                    }
                }
            }
        }
    }

    ItemDialog {
        id: attackDialog
        title: "Attack"
        fields: [
            { key: "name", label: "Name", type: "text" },
            { key: "ability", label: "Ability", type: "choice", options: App.catalog.abilities },
            { key: "proficient", label: "Proficient", type: "bool" },
            { key: "damage", label: "Damage dice", type: "text" },
            { key: "add_ability_to_damage", label: "Add ability modifier to damage", type: "bool" },
            { key: "bonus", label: "Magic bonus", type: "int", from: -10, to: 10 },
            { key: "damage_type", label: "Damage type", type: "text" },
            { key: "mastery", label: "Weapon mastery", type: "choice", options: tab.masteryOptions },
            { key: "notes", label: "Notes", type: "area" }
        ]
    }

    Kirigami.PromptDialog {
        id: longRestDialog
        title: "Take a Long Rest?"
        subtitle: "Restores all HP, hit dice, spell slots and feature uses, and removes one level of Exhaustion."
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: App.sheet.longRest()
    }
}
