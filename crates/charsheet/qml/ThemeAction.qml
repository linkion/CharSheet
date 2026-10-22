import QtQuick
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// The overflow-menu entry for choosing the app's look.
Kirigami.Action {
    text: "Theme"
    icon.name: "color-management"
    displayHint: Kirigami.DisplayHint.AlwaysHide

    Kirigami.Action {
        text: "Stone (2024 sheet)"
        checkable: true
        checked: ThemeSettings.mode === "stone"
        onTriggered: ThemeSettings.choose("stone")
    }
    Kirigami.Action {
        text: "Parchment (Player's Handbook)"
        checkable: true
        checked: ThemeSettings.mode === "parchment"
        onTriggered: ThemeSettings.choose("parchment")
    }
    Kirigami.Action {
        text: "System"
        checkable: true
        checked: ThemeSettings.mode === "system"
        onTriggered: ThemeSettings.choose("system")
    }
}
