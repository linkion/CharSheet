import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import io.github.linkion.charsheet

// A multi-line text box bound to a character field. Shows the text rendered as Markdown;
// clicking it opens the source for editing, saved when it loses focus. Written on graph
// paper in the book face in the decorated themes, like the sheet's free-text boxes.
Item {
    id: area

    required property string pointer
    property string value
    property string placeholderText
    property bool editing: false

    Layout.fillWidth: true
    Layout.minimumHeight: implicitHeight
    implicitHeight: editing ? editor.implicitHeight : view.implicitHeight

    function edit() {
        editing = true
        editor.forceActiveFocus()
        editor.cursorPosition = editor.length
    }

    QQC2.TextArea {
        id: view
        visible: !area.editing
        anchors.fill: parent
        text: area.value
        textFormat: TextEdit.MarkdownText
        readOnly: true
        wrapMode: TextEdit.Wrap
        font.family: Style.proseFamily
        placeholderText: area.placeholderText || "Click to write…"
        onLinkActivated: link => App.openLink(link)

        HoverHandler {
            cursorShape: view.hoveredLink ? Qt.PointingHandCursor : Qt.IBeamCursor
        }
        TapHandler {
            onTapped: if (!view.hoveredLink) area.edit()
        }
        // Inside the background, so the lines sit under the text.
        GridPaper {
            parent: view.background
            anchors.fill: parent
            anchors.margins: 2
        }
    }

    QQC2.TextArea {
        id: editor
        visible: area.editing
        anchors.fill: parent
        text: area.value
        wrapMode: TextEdit.Wrap
        font.family: Style.proseFamily
        placeholderText: area.placeholderText || "Markdown supported"
        Keys.onEscapePressed: focus = false
        onEditingFinished: {
            if (text !== area.value)
                App.set(area.pointer, text)
            area.editing = false
        }

        GridPaper {
            parent: editor.background
            anchors.fill: parent
            anchors.margins: 2
        }
    }
}
