import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// A titled card used to group parts of the sheet.
SheetCard {
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
            // Balances the actions so a centered title stays centered.
            Item {
                visible: Style.decorated
                implicitWidth: actions.implicitWidth
            }
            FrameTitle {
                text: section.title
            }
            RowLayout {
                id: actions
                spacing: 0
                Repeater {
                    model: section.headerActions
                    QQC2.ToolButton {
                        required property var modelData
                        action: modelData
                    }
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
