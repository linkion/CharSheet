import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Dialogs
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

Kirigami.ScrollablePage {
    id: page

    title: "Characters"

    actions: [
        Kirigami.Action {
            text: "New Character"
            icon.name: "list-add"
            onTriggered: newDialog.open()
        },
        Kirigami.Action {
            text: "Import…"
            icon.name: "document-import"
            onTriggered: importDialog.open()
        },
        ThemeAction {}
    ]

    background: SheetBackground {}

    Kirigami.CardsLayout {
        id: cards

        Repeater {
            model: App.entries

            Kirigami.Card {
                id: card
                required property var modelData

                background.visible: !Style.decorated
                padding: Style.decorated ? Kirigami.Units.largeSpacing + 4 : Kirigami.Units.largeSpacing

                SheetFrame {
                    parent: card
                    z: -1
                    anchors.fill: parent
                    visible: Style.decorated
                }

                banner.title: modelData.name
                banner.titleIcon: "user-identity"
                onClicked: applicationWindow().openCharacter(modelData.id)
                showClickFeedback: true

                contentItem: ColumnLayout {
                    QQC2.Label {
                        text: "Level " + modelData.level + (modelData.species ? " · " + modelData.species : "")
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                    }
                    QQC2.Label {
                        text: modelData.class_summary
                        opacity: 0.7
                        Layout.fillWidth: true
                        wrapMode: Text.WordWrap
                    }
                }

                actions: [
                    Kirigami.Action {
                        text: "Open"
                        icon.name: "document-open"
                        onTriggered: applicationWindow().openCharacter(modelData.id)
                    },
                    Kirigami.Action {
                        text: "Duplicate"
                        icon.name: "edit-copy"
                        displayHint: Kirigami.DisplayHint.AlwaysHide
                        onTriggered: App.library.duplicate(modelData.id)
                    },
                    Kirigami.Action {
                        text: "Export…"
                        icon.name: "document-export"
                        displayHint: Kirigami.DisplayHint.AlwaysHide
                        onTriggered: {
                            exportDialog.characterId = modelData.id
                            exportDialog.selectedFile = "file:///" + modelData.name.replace(/[^\w\- ]/g, "") + ".json"
                            exportDialog.open()
                        }
                    },
                    Kirigami.Action {
                        text: "Delete"
                        icon.name: "edit-delete"
                        displayHint: Kirigami.DisplayHint.AlwaysHide
                        onTriggered: {
                            deleteDialog.characterId = modelData.id
                            deleteDialog.subtitle = "\"" + modelData.name + "\" will be permanently deleted."
                            deleteDialog.open()
                        }
                    }
                ]
            }
        }
    }

    Kirigami.PlaceholderMessage {
        parent: page
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: App.entries.length === 0
        icon.name: "user-identity"
        text: "No characters yet"
        explanation: "Create a character or import one from a file."
        helpfulAction: Kirigami.Action {
            text: "New Character"
            icon.name: "list-add"
            onTriggered: newDialog.open()
        }
    }

    Kirigami.PromptDialog {
        id: newDialog
        title: "New Character"
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onOpened: {
            nameField.text = ""
            nameField.forceActiveFocus()
        }
        onAccepted: {
            const id = App.library.create(nameField.text)
            if (id.length > 0)
                applicationWindow().openCharacter(id)
        }
        QQC2.TextField {
            id: nameField
            placeholderText: "Character name"
            onAccepted: newDialog.accept()
        }
    }

    Kirigami.PromptDialog {
        id: deleteDialog
        property string characterId
        title: "Delete Character?"
        standardButtons: Kirigami.Dialog.Ok | Kirigami.Dialog.Cancel
        onAccepted: App.library.remove(characterId)
    }

    FileDialog {
        id: importDialog
        title: "Import Character"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Character files (*.json)", "All files (*)"]
        onAccepted: {
            const id = App.library.importFile(selectedFile)
            if (id.length > 0)
                applicationWindow().showPassiveNotification("Character imported")
        }
    }

    FileDialog {
        id: exportDialog
        property string characterId
        title: "Export Character"
        fileMode: FileDialog.SaveFile
        defaultSuffix: "json"
        nameFilters: ["Character files (*.json)"]
        onAccepted: {
            if (App.library.exportFile(characterId, selectedFile))
                applicationWindow().showPassiveNotification("Character exported")
        }
    }
}
