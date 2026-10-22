import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// An entry of derived.actions: what it is, what it has left, and a button to use it.
RowLayout {
    id: row

    required property var action

    Layout.fillWidth: true

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 0

        QQC2.Label {
            text: row.action.owner === "spell" ? row.action.name : row.action.name + "  —  " + row.action.source
            font.bold: true
            Layout.fillWidth: true
            elide: Text.ElideRight
        }
        QQC2.Label {
            readonly property string tags: [row.action.owner === "spell" ? row.action.source : "",
                                            row.action.concentration ? "Concentration" : "",
                                            row.action.roll ? "Rolls " + row.action.roll : ""].filter(s => s).join(" · ")
            visible: text.length > 0
            text: [tags, row.action.description].filter(s => s).join("  —  ")
            opacity: 0.7
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
            Layout.fillWidth: true
        }
    }
    QQC2.Label {
        visible: !!row.action.resource
        text: row.action.resource ? row.action.resource.left + (row.action.resource.label === "left" ? " left" : " / " + row.action.resource.max) : ""
        font.bold: true
        color: row.action.available ? Kirigami.Theme.textColor : Kirigami.Theme.negativeTextColor
        QQC2.ToolTip.text: row.action.resource ? row.action.resource.label : ""
        QQC2.ToolTip.visible: hover.hovered
        HoverHandler { id: hover }
    }
    ActionButton {
        entry: row.action
    }
}
