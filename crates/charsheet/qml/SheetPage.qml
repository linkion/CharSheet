import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

Kirigami.Page {
    id: page

    title: App.c.name || ""
    padding: 0

    Component.onDestruction: App.library.refresh()

    actions: [
        Kirigami.Action {
            text: App.rollMode === "advantage" ? "Advantage" : App.rollMode === "disadvantage" ? "Disadvantage" : "Normal Rolls"
            icon.name: App.rollMode === "advantage" ? "arrow-up" : App.rollMode === "disadvantage" ? "arrow-down" : "roll"
            tooltip: "Advantage or disadvantage for d20 rolls"
            Kirigami.Action {
                text: "Normal"
                checkable: true
                checked: App.rollMode === "normal"
                onTriggered: App.rollMode = "normal"
            }
            Kirigami.Action {
                text: "Advantage"
                checkable: true
                checked: App.rollMode === "advantage"
                onTriggered: App.rollMode = "advantage"
            }
            Kirigami.Action {
                text: "Disadvantage"
                checkable: true
                checked: App.rollMode === "disadvantage"
                onTriggered: App.rollMode = "disadvantage"
            }
        },
        Kirigami.Action {
            text: "Roll Log"
            icon.name: "view-history"
            onTriggered: applicationWindow().showRollLog()
        },
        Kirigami.Action {
            text: "Short Rest"
            icon.name: "chronometer-pause"
            displayHint: Kirigami.DisplayHint.AlwaysHide
            onTriggered: {
                App.sheet.shortRest()
                applicationWindow().showPassiveNotification("Short rest: Pact slots, short-rest features and item charges restored. Spend hit dice on the Combat tab.")
            }
        },
        Kirigami.Action {
            text: "Long Rest"
            icon.name: "go-sleep"
            displayHint: Kirigami.DisplayHint.AlwaysHide
            onTriggered: longRestDialog.open()
        }
    ]

    header: QQC2.TabBar {
        id: tabs
        Component.onCompleted: {
            currentIndex = App.requestedTab
            App.requestedTab = 0
        }
        QQC2.TabButton { text: "Main" }
        QQC2.TabButton { text: "Combat" }
        QQC2.TabButton { text: "Spells" }
        QQC2.TabButton { text: "Inventory" }
        QQC2.TabButton { text: "Features" }
        QQC2.TabButton { text: "Notes" }
        QQC2.TabButton { text: "Build" }
    }

    StackLayout {
        anchors.fill: parent
        currentIndex: tabs.currentIndex

        MainTab {}
        CombatTab {}
        SpellsTab {}
        InventoryTab {}
        FeaturesTab {}
        NotesTab {}
        BuildTab {}
    }

    Kirigami.PromptDialog {
        id: longRestDialog
        title: "Take a Long Rest?"
        subtitle: "Restores all HP, hit dice, spell slots, feature uses, item charges and free casts, ends active effects, and removes one level of Exhaustion."
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: App.sheet.longRest()
    }
}
