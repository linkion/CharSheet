import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

Kirigami.OverlayDrawer {
    edge: Qt.application.layoutDirection === Qt.RightToLeft ? Qt.LeftEdge : Qt.RightEdge
    modal: true
    width: Math.min(Kirigami.Units.gridUnit * 20, parent ? parent.width : 0)
    leftPadding: 0
    rightPadding: 0
    topPadding: 0
    bottomPadding: 0

    contentItem: ColumnLayout {
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing

            Kirigami.Heading {
                text: "Roll log"
                level: 2
                Layout.fillWidth: true
            }
            QQC2.ToolButton {
                icon.name: "edit-clear-history"
                text: "Clear"
                display: QQC2.AbstractButton.IconOnly
                enabled: App.rolls.length > 0
                onClicked: App.sheet.clearRolls()
                QQC2.ToolTip.text: text
                QQC2.ToolTip.visible: hovered
            }
        }

        DiceBox {
            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.largeSpacing
            Layout.rightMargin: Kirigami.Units.largeSpacing
            Layout.bottomMargin: Kirigami.Units.largeSpacing
        }

        Kirigami.Separator {
            Layout.fillWidth: true
        }

        QQC2.ScrollView {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                model: App.rolls
                clip: true

                delegate: QQC2.ItemDelegate {
                    required property var modelData
                    width: ListView.view.width
                    contentItem: RowLayout {
                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 0
                            QQC2.Label {
                                text: modelData.label
                                font.bold: true
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                            QQC2.Label {
                                text: modelData.expression + (modelData.mode !== "normal" ? " (" + modelData.mode + ")" : "") + "  " + modelData.breakdown
                                opacity: 0.7
                                elide: Text.ElideRight
                                Layout.fillWidth: true
                            }
                        }
                        Kirigami.Heading {
                            text: modelData.total
                            level: 2
                            color: modelData.natural === 20 ? Kirigami.Theme.positiveTextColor
                                 : modelData.natural === 1 ? Kirigami.Theme.negativeTextColor
                                 : Kirigami.Theme.textColor
                        }
                    }
                }

                Kirigami.PlaceholderMessage {
                    anchors.centerIn: parent
                    width: parent.width - Kirigami.Units.gridUnit * 2
                    visible: App.rolls.length === 0
                    text: "No rolls yet"
                    explanation: "Click any modifier on the sheet to roll it."
                }
            }
        }
    }
}
