import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// One attack with an equipped weapon: to-hit and damage rolls, damage riders, and the
// cylinder and ammunition for weapons that reload.
ColumnLayout {
    id: row

    required property int attackIndex
    readonly property var a: (App.d.weapon_attacks || [])[attackIndex] || ({ riders: [], notes: [] })
    readonly property var weapon: App.c.inventory?.[a.item]?.weapon ?? ({})
    readonly property bool firesAmmo: a.ranged && !a.thrown
    readonly property var ammoOptions: [{ key: "", name: "Standard (not tracked)" }].concat(
        (App.c.inventory || []).filter(i => i.kind === "ammunition")
            .map(i => ({ key: i.name, name: i.name + " (" + i.quantity + ")" })))
    // Riders left out of damage rolls by unticking their chip; kept across turns so an
    // untargeted Hunter's Mark stays off until ticked again.
    property var excluded: []

    function rollDamage(crit) {
        App.showRoll(App.sheet.weaponDamage(attackIndex, crit, JSON.stringify(excluded)))
    }

    Layout.fillWidth: true
    spacing: 0

    RowLayout {
        Layout.fillWidth: true

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0
            QQC2.Label {
                text: row.a.name ?? ""
                font.bold: true
                Layout.fillWidth: true
                elide: Text.ElideRight
            }
            QQC2.Label {
                text: [row.a.range === "melee" ? "Melee" : "Range " + row.a.range, row.a.damage_type,
                       row.a.mastery ? "Mastery: " + row.a.mastery : ""].filter(s => s).join(" · ")
                opacity: 0.7
                Layout.fillWidth: true
                elide: Text.ElideRight
            }
        }
        QQC2.Button {
            text: App.signed(row.a.to_hit ?? 0)
            font.bold: true
            enabled: !row.firesAmmo || row.a.capacity === 0 || row.a.loaded > 0
            onClicked: App.showRoll(App.sheet.weaponAttack(row.attackIndex, App.rollMode))
            QQC2.ToolTip.text: "Roll to hit" + (row.firesAmmo ? " (fires a shot)" : "")
            QQC2.ToolTip.visible: hovered
        }
        QQC2.Button {
            text: row.a.damage ?? ""
            flat: true
            onClicked: row.rollDamage(false)
            QQC2.ToolTip.text: "Roll damage with the ticked riders"
            QQC2.ToolTip.visible: hovered
        }
        QQC2.ToolButton {
            text: "Crit"
            onClicked: row.rollDamage(true)
            QQC2.ToolTip.text: "Roll critical damage (dice doubled)"
            QQC2.ToolTip.visible: hovered
        }
    }

    Flow {
        visible: row.a.riders.length > 0
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Repeater {
            model: row.a.riders
            Kirigami.Chip {
                required property var modelData
                text: "+" + modelData.dice + " " + modelData.name + (modelData.used ? " (used)" : "")
                checkable: true
                closable: false
                enabled: !modelData.used
                checked: !modelData.used && !row.excluded.includes(modelData.name)
                onToggled: {
                    const rest = row.excluded.filter(n => n !== modelData.name)
                    row.excluded = checked ? rest : rest.concat([modelData.name])
                }
                QQC2.ToolTip.text: modelData.once_per_turn ? "Once per turn; resets on New Turn" : "Added to every hit while ticked"
                QQC2.ToolTip.visible: hovered
            }
        }
    }

    RowLayout {
        visible: row.firesAmmo
        Layout.fillWidth: true

        QQC2.Label {
            visible: row.a.capacity > 0
            text: "Loaded " + row.a.loaded + " / " + row.a.capacity
            color: row.a.loaded === 0 ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
        }
        QQC2.Button {
            visible: row.a.capacity > 0
            text: "Reload"
            icon.name: "view-refresh"
            enabled: row.a.loaded < row.a.capacity
            onClicked: App.sheet.reload(row.a.item)
        }
        QQC2.ComboBox {
            model: row.ammoOptions
            textRole: "name"
            valueRole: "key"
            currentIndex: row.ammoOptions.findIndex(o => o.key === (row.weapon.ammo ?? ""))
            onActivated: App.set("/inventory/" + row.a.item + "/weapon/ammo", currentValue)
            QQC2.ToolTip.text: "Ammunition"
            QQC2.ToolTip.visible: hovered
        }
        Item { Layout.fillWidth: true }
    }

    Repeater {
        model: row.a.notes
        QQC2.Label {
            required property var modelData
            text: "• " + modelData
            wrapMode: Text.WordWrap
            opacity: 0.8
            Layout.fillWidth: true
        }
    }
}
