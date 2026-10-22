pragma Singleton
import QtQuick
import io.github.porave.charsheet

// The decorated themes' colors and fonts. The color scheme (cpp/theme.h) already covers
// text, controls and backgrounds; these are the extras the sheet decorations need.
QtObject {
    readonly property string mode: ThemeSettings.mode
    // False for the System theme, which draws plain Kirigami cards instead of sheet frames.
    readonly property bool decorated: mode !== "system"
    readonly property bool parchment: mode === "parchment"

    readonly property color heading: parchment ? "#58180D" : "#2A2A2A"
    readonly property color frameOuter: parchment ? "#58180D" : "#6A6A6A"
    readonly property color frameInner: parchment ? "#C9AD6A" : "#ABABAB"
    readonly property color panel: parchment ? "#FDF1DC" : "#FFFFFF"
    // Grid paper and other faint lines drawn on panels.
    readonly property color faint: parchment ? "#EBDDBC" : "#E6E6E3"
    readonly property url texture: parchment ? "qrc:/theme/parchment.png" : "qrc:/theme/stone.png"

    // The application font (set in cpp/theme.h), for labels that pick a family explicitly.
    readonly property string uiFamily: decorated ? "Scaly Sans Remake" : ThemeSettings.systemFamily
    // Frame titles and small captions ("MODIFIER", "SCORE").
    readonly property string labelFamily: decorated ? "Scaly Sans Caps" : ThemeSettings.systemFamily
    // Notes and descriptions.
    readonly property string proseFamily: decorated ? "Bookinsanity Remake" : ThemeSettings.systemFamily
    // The character name and large numbers.
    readonly property string displayFamily: decorated ? "Nodesto Caps Condensed" : ThemeSettings.systemFamily
}
