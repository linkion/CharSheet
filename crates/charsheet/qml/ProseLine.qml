import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import io.github.porave.charsheet

// A one-line summary: generated text, then the first paragraph of the player's Markdown,
// with inline styling kept so it can still be cut short.
QQC2.Label {
    id: line

    // Plain text from the app, such as "Weapon · 1d8 slashing".
    property string prefix
    // StyledText from the app that can carry links, as built by App.ruleLink.
    property string links
    property string source

    readonly property string styled: [links, App.escaped(prefix), Markdown.toLine(source)].filter(s => s).join("  —  ")

    visible: styled.length > 0
    text: styled
    textFormat: Text.StyledText
    elide: Text.ElideRight
    Layout.fillWidth: true
    onLinkActivated: link => App.openLink(link)

    HoverHandler {
        cursorShape: line.hoveredLink ? Qt.PointingHandCursor : undefined
    }
}
