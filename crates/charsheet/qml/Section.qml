import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// A titled card used to group parts of the sheet.
Kirigami.AbstractCard {
    id: section

    property string title
    property list<QtObject> headerActions
    default property alias content: body.data

    Layout.fillWidth: true
    Layout.alignment: Qt.AlignTop

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            Layout.fillWidth: true
            Kirigami.Heading {
                text: section.title
                level: 3
                Layout.fillWidth: true
            }
            Repeater {
                model: section.headerActions
                QQC2.ToolButton {
                    required property var modelData
                    action: modelData
                }
            }
        }

        ColumnLayout {
            id: body
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
        }
    }
}
