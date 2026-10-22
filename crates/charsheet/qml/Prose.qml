import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import io.github.porave.charsheet

// Free text the player wrote, rendered as Markdown in the book face.
QQC2.Label {
    id: prose

    property string source

    text: source
    textFormat: Text.MarkdownText
    wrapMode: Text.WordWrap
    font.family: Style.proseFamily
    Layout.fillWidth: true
    onLinkActivated: link => Qt.openUrlExternally(link)

    HoverHandler {
        cursorShape: prose.hoveredLink ? Qt.PointingHandCursor : undefined
    }
}
