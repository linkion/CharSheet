import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// The play surface: hit points, the turn, active effects, attacks and everything else the
// character can do.
SheetTab {
    id: tab

    readonly property var activationGroups: [
        { key: "action", title: "Actions" },
        { key: "bonus_action", title: "Bonus Actions" },
        { key: "reaction", title: "Reactions" },
        { key: "free", title: "Free" },
        { key: "other", title: "Other" }
    ]

    function takeDamage(amount) {
        if (amount <= 0)
            return
        App.sheet.damage(amount)
        const concentrating = App.d.concentrating
        if (!concentrating)
            return
        const dc = Math.min(30, Math.max(10, Math.floor(amount / 2)))
        applicationWindow().showPassiveNotification("Concentrating on " + concentrating + ": CON save DC " + dc,
            "long", "Roll Save", () => App.rollCheck({ kind: "concentration", dc: dc }))
    }

    GridLayout {
        Layout.fillWidth: true
        Layout.columnSpan: tab.columns
        columns: 4
        columnSpacing: Kirigami.Units.largeSpacing

        StatTile { label: "Armor Class"; value: App.d.armor_class ?? ""; shape: "shield"; Layout.fillHeight: true }
        StatTile { label: "Initiative"; rollable: true; rollModifier: App.d.initiative ?? 0; rollTarget: ({ kind: "initiative" }); Layout.fillHeight: true }
        StatTile { label: "Speed"; value: (App.d.speed ?? 0) + " ft"; Layout.fillHeight: true }
        StatTile { label: "Proficiency"; value: App.signed(App.d.proficiency_bonus ?? 0); Layout.fillHeight: true }
    }

    Section {
        title: "Hit Points"

        // Current, temporary and maximum hit points, as on the sheet.
        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            // The model holds no theme colors: a model that changed with the theme would be
            // rebuilt while Kirigami is still notifying its delegates, which crashes.
            Repeater {
                model: [
                    { key: "current", label: "Current", large: true },
                    { key: "temp", label: "Temp", large: false },
                    { key: "max", label: "Max", large: false }
                ]

                RowLayout {
                    required property var modelData
                    required property int index
                    Layout.fillWidth: modelData.large
                    spacing: Kirigami.Units.largeSpacing

                    Rectangle {
                        visible: index > 0
                        implicitWidth: 1.5
                        color: Style.decorated ? Style.frameOuter : Kirigami.Theme.disabledTextColor
                        Layout.fillHeight: true
                    }
                    ColumnLayout {
                        spacing: 0
                        Layout.fillWidth: true
                        QQC2.Label {
                            readonly property int current: App.c.hp?.current ?? 0
                            readonly property int temp: App.c.hp?.temp ?? 0
                            text: modelData.key === "current" ? current : modelData.key === "temp" ? temp : (App.d.max_hp ?? 0)
                            color: modelData.key === "current" && current === 0 ? Kirigami.Theme.negativeTextColor
                                 : modelData.key === "temp" ? (temp > 0 ? Kirigami.Theme.positiveTextColor : Kirigami.Theme.disabledTextColor)
                                 : Kirigami.Theme.textColor
                            font.family: Style.displayFamily
                            font.bold: !Style.decorated
                            font.pointSize: Kirigami.Theme.defaultFont.pointSize * (modelData.large ? 2.6 : 1.7)
                        }
                        Rectangle {
                            implicitHeight: 1
                            color: Style.decorated ? Style.frameInner : Kirigami.Theme.disabledTextColor
                            Layout.fillWidth: true
                        }
                        QQC2.Label {
                            text: modelData.label
                            opacity: 0.75
                            font.family: Style.labelFamily
                            font.capitalization: Font.AllUppercase
                            font.letterSpacing: 1
                            font.pointSize: Kirigami.Theme.smallFont.pointSize
                        }
                    }
                }
            }
        }

        QQC2.ProgressBar {
            Layout.fillWidth: true
            from: 0
            to: App.d.max_hp ?? 1
            value: App.c.hp?.current ?? 0
        }

        QQC2.Label {
            visible: (App.d.resistances || []).length > 0
            text: "Resistant to " + (App.d.resistances || []).map(r => r === "all" ? "all damage" : r).join(", ")
            color: Kirigami.Theme.positiveTextColor
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
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
                onClicked: tab.takeDamage(amount.value)
            }
            QQC2.Button {
                text: "Resisted (½)"
                onClicked: tab.takeDamage(Math.floor(amount.value / 2))
                QQC2.ToolTip.text: "Take half damage, rounded down"
                QQC2.ToolTip.visible: hovered
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
                onClicked: App.rollCheck({ kind: "death_save" })
            }
        }
    }

    Section {
        title: "Turn & Effects"

        headerActions: [
            Kirigami.Action {
                text: "Add Effect"
                icon.name: "list-add"
                tooltip: "Add an effect by hand, such as a Bless an ally cast on you"
                onTriggered: effectDialog.create("/effects")
            }
        ]

        RowLayout {
            Layout.fillWidth: true
            Kirigami.Heading {
                text: (App.c.round ?? 0) > 0 ? "Round " + App.c.round : "Not in combat"
                level: 2
                Layout.fillWidth: true
            }
            QQC2.Button {
                text: (App.c.round ?? 0) > 0 ? "New Turn" : "Start Combat"
                icon.name: "media-skip-forward"
                onClicked: App.sheet.newTurn()
                QQC2.ToolTip.text: "Start your turn: once-per-turn riders reset and timed effects count down"
                QQC2.ToolTip.visible: hovered
            }
            QQC2.Button {
                visible: (App.c.round ?? 0) > 0
                text: "End Combat"
                icon.name: "media-playback-stop"
                onClicked: App.sheet.endCombat()
                QQC2.ToolTip.text: "Reset the round and end effects that last a number of turns"
                QQC2.ToolTip.visible: hovered
            }
        }

        // Switchable modifiers other than the weapons' own, which sit on their attack rows.
        Flow {
            id: switches
            // QML's JavaScript has no Array.prototype.flatMap.
            readonly property var weaponPointers: (App.d.weapon_attacks || [])
                .reduce((pointers, a) => pointers.concat((a.toggles || []).map(t => t.pointer)), [])
            readonly property var toggles: (App.d.toggles || [])
                .filter(t => !weaponPointers.includes(t.pointer))

            visible: toggles.length > 0
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Repeater {
                model: switches.toggles
                QQC2.Switch {
                    required property var modelData
                    text: modelData.label + "  (" + modelData.source + ")"
                    checked: modelData.enabled
                    onToggled: App.set(modelData.pointer + "/enabled", checked)
                    QQC2.ToolTip.text: modelData.description
                    QQC2.ToolTip.visible: hovered
                }
            }
        }

        QQC2.Label {
            visible: !!App.d.concentrating
            text: "Concentrating on " + App.d.concentrating
            font.bold: true
            color: Kirigami.Theme.neutralTextColor
        }

        Repeater {
            model: App.c.effects

            SheetRow {
                required property var modelData
                required property int index
                alternate: index % 2 === 1
                clickable: true
                onClicked: App.detailsRequested("/effects", index)

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        text: modelData.name
                              + (modelData.concentration ? "  [C]" : "")
                              + (modelData.rounds > 0 ? "  · " + modelData.rounds + (modelData.rounds === 1 ? " turn" : " turns") : "")
                              + (modelData.source ? "  —  " + modelData.source : "")
                        font.bold: true
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                    QQC2.Label {
                        text: (App.d.effect_summaries || [])[index] ?? ""
                        visible: text.length > 0
                        wrapMode: Text.WordWrap
                        opacity: 0.8
                        Layout.fillWidth: true
                    }
                    Prose {
                        visible: source.length > 0
                        source: modelData.notes ?? ""
                        font.family: Style.uiFamily
                        opacity: 0.8
                    }
                }
                QQC2.ToolButton {
                    icon.name: "dialog-close"
                    onClicked: App.sheet.endEffect(index)
                    QQC2.ToolTip.text: "End effect"
                    QQC2.ToolTip.visible: hovered
                }
            }
        }

        QQC2.Label {
            visible: (App.c.effects || []).length === 0
            text: "No active effects. Spells and actions add theirs here."
            opacity: 0.7
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }

        Kirigami.Separator {
            visible: (App.d.notes_in_force || []).length > 0
            Layout.fillWidth: true
        }

        Repeater {
            model: App.d.notes_in_force
            Prose {
                required property var modelData
                source: "• " + modelData.text + "  (" + modelData.source + ")"
                font.family: Style.uiFamily
                opacity: 0.8
            }
        }
    }

    Section {
        title: "Attacks"
        Layout.columnSpan: tab.columns

        headerActions: [
            Kirigami.Action {
                text: "Add Attack"
                icon.name: "list-add"
                tooltip: "Add an attack by hand. Equipped weapons in the Inventory show up here on their own."
                onTriggered: attackDialog.create("/attacks")
            }
        ]

        // Indexed by count so the rider chips keep their state across edits.
        Repeater {
            model: (App.d.weapon_attacks || []).length

            SheetRow {
                required property int index
                alternate: index % 2 === 1
                clickable: true
                onClicked: App.detailsRequested("/inventory", App.d.weapon_attacks[index].item)
                WeaponAttackRow { attackIndex: index }
            }
        }

        Repeater {
            model: App.c.attacks

            SheetRow {
                required property var modelData
                required property int index
                readonly property var derived: App.d.attacks[index]
                alternate: ((App.d.weapon_attacks || []).length + index) % 2 === 1
                clickable: true
                onClicked: App.detailsRequested("/attacks", index)

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0
                    QQC2.Label {
                        text: modelData.name
                        font.bold: true
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                    ProseLine {
                        links: [App.escaped(modelData.damage_type ?? ""),
                                modelData.mastery ? "Mastery: " + App.ruleLink(modelData.mastery, ["mastery_property"]) : ""]
                               .filter(s => s).join(" · ")
                        source: modelData.notes ?? ""
                        opacity: 0.7
                    }
                }
                RollButton {
                    modifier: derived ? derived.to_hit : 0
                    target: ({ kind: "manual_attack", attack: index })
                    rollLabel: modelData.name + " attack"
                }
                QQC2.Button {
                    text: derived ? derived.damage : ""
                    flat: true
                    onClicked: App.rollCheck({ kind: "manual_damage", attack: index, crit: false })
                    QQC2.ToolTip.text: "Roll damage"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.ToolButton {
                    text: "Crit"
                    onClicked: App.rollCheck({ kind: "manual_damage", attack: index, crit: true })
                    QQC2.ToolTip.text: "Roll critical damage (dice doubled)"
                    QQC2.ToolTip.visible: hovered
                }
            }
        }

        QQC2.Label {
            visible: (App.c.attacks || []).length === 0 && (App.d.weapon_attacks || []).length === 0
            text: "No attacks yet. Equip a weapon in the Inventory, or add one by hand."
            opacity: 0.7
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }

    Section {
        title: "Actions, Spells & Items"
        Layout.columnSpan: tab.columns

        Repeater {
            model: tab.activationGroups

            ColumnLayout {
                id: group
                required property var modelData
                readonly property var actions: (App.d.actions || []).filter(a => a.activation === modelData.key)
                visible: actions.length > 0
                Layout.fillWidth: true

                Kirigami.Heading {
                    text: group.modelData.title
                    level: 4
                }
                Repeater {
                    model: group.actions
                    ActionRow {
                        required property var modelData
                        required property int index
                        action: modelData
                        alternate: index % 2 === 1
                    }
                }
            }
        }

        QQC2.Label {
            visible: (App.d.actions || []).length === 0
            text: "Give items and features actions in their edit dialogs, and prepared spells an effect or a roll, and they show up here."
            opacity: 0.7
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
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
                    onClicked: App.report(App.sheet.spendHitDie(index))
                }
            }
        }

        RowLayout {
            QQC2.Button {
                text: "Short Rest"
                icon.name: "chronometer-pause"
                onClicked: {
                    App.sheet.shortRest()
                    applicationWindow().showPassiveNotification("Pact slots, short-rest features and item charges restored")
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
        title: "Conditions"

        RowLayout {
            ProseLine {
                links: App.ruleLink("Exhaustion", ["condition"])
                Layout.fillWidth: false
            }
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

                    readonly property string rule: Srd.find(modelData.name, JSON.stringify(["condition"]))
                    QQC2.ToolTip.text: "Right-click for the rules"
                    QQC2.ToolTip.visible: hovered
                    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
                    // A TapHandler would let the chip see the click and toggle as well.
                    MouseArea {
                        anchors.fill: parent
                        acceptedButtons: Qt.RightButton
                        onClicked: App.ruleRequested(parent.rule)
                    }
                }
            }
        }

        // The conditions in force, as links for screens with no right click.
        ProseLine {
            links: App.catalog.conditions
                   .filter(c => (App.c.conditions || []).includes(c.key))
                   .map(c => App.ruleLink(c.name, ["condition"])).join(" · ")
        }
    }

    Connections {
        target: App
        function onEditRequested(listPointer, index) {
            if (listPointer === "/attacks")
                attackDialog.edit(listPointer, index)
            else if (listPointer === "/effects")
                effectDialog.edit(listPointer, index)
        }
    }

    ItemDialog {
        id: attackDialog
        noun: "Attack"
        fields: [
            { key: "name", label: "Name", type: "text", placeholder: "e.g. Unarmed Strike" },
            { type: "row", fields: [
                { key: "ability", label: "Ability", type: "choice", options: App.catalog.abilities },
                { key: "bonus", label: "Magic bonus", type: "int", from: -10, to: 10 }
            ] },
            { type: "row", fields: [
                { key: "damage", label: "Damage", type: "text", placeholder: "1d6" },
                { key: "damage_type", label: "Type", type: "text", placeholder: "bludgeoning" }
            ] },
            { label: "", type: "flags", flags: [
                { key: "proficient", label: "Proficient" },
                { key: "add_ability_to_damage", label: "Add ability modifier to damage" }
            ] },
            { key: "mastery", label: "Weapon mastery", type: "choice", options: App.masteryOptions },
            { label: "Notes", type: "section" },
            { key: "notes", label: "", type: "area" }
        ]
    }

    ItemDialog {
        id: effectDialog
        noun: "Effect"
        fields: [
            { key: "name", label: "Name", type: "text" },
            { key: "source", label: "Source", type: "text", placeholder: "e.g. Bless from the cleric" },
            { key: "rounds", label: "Turns left (0 = until removed)", type: "int", from: 0, to: 1000 },
            { key: "concentration", label: "Concentration", type: "bool" },
            { label: "Notes", type: "section" },
            { key: "notes", label: "", type: "area" },
            { label: "Modifiers", type: "section", badge: count("modifiers") },
            { key: "modifiers", label: "Modifiers", type: "modifiers" }
        ]
    }

    Kirigami.PromptDialog {
        id: longRestDialog
        title: "Take a Long Rest?"
        subtitle: "Restores all HP, hit dice, spell slots, feature uses, item charges and free casts, rolls for charges regained at dawn, ends active effects, and removes one level of Exhaustion."
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: App.sheet.longRest()
    }
}
