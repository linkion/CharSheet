import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

SheetTab {
    id: tab

    Section {
        title: "Features & Traits"
        Layout.columnSpan: tab.columns

        headerActions: [
            Kirigami.Action {
                text: "Add Feature"
                icon.name: "list-add"
                onTriggered: {
                    if (App.append("/features", {}))
                        featureDialog.edit("/features", App.c.features.length - 1)
                }
            }
        ]

        Repeater {
            model: App.c.features

            ColumnLayout {
                required property var modelData
                required property int index
                readonly property string summary: (App.d.feature_summaries || [])[index] ?? ""
                readonly property var actions: (App.d.actions || []).filter(a => a.owner === "feature" && a.index === index)
                Layout.fillWidth: true
                spacing: 0

                RowLayout {
                    Layout.fillWidth: true
                    QQC2.Label {
                        text: modelData.name
                        font.bold: true
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                    QQC2.Label {
                        visible: modelData.source.length > 0
                        text: modelData.source
                        opacity: 0.7
                    }
                    PipRow {
                        visible: modelData.uses_max > 0
                        total: Math.min(modelData.uses_max, 20)
                        used: modelData.uses_spent
                        onUsedEdited: count => App.set("/features/" + index + "/uses_spent", count)
                    }
                    Repeater {
                        model: actions
                        ActionButton {
                            required property var modelData
                            entry: modelData
                        }
                    }
                    QQC2.ToolButton {
                        icon.name: "document-edit"
                        onClicked: featureDialog.edit("/features", index)
                        QQC2.ToolTip.text: "Edit"
                        QQC2.ToolTip.visible: hovered
                    }
                }
                Prose {
                    visible: modelData.description.length > 0
                    source: modelData.description
                    opacity: 0.85
                }
                QQC2.Label {
                    visible: summary.length > 0
                    text: summary
                    wrapMode: Text.WordWrap
                    color: Kirigami.Theme.positiveTextColor
                    Layout.fillWidth: true
                }
                QQC2.Label {
                    visible: modelData.uses_max > 0 && modelData.recharge !== "none"
                    text: "Recharges on a " + (modelData.recharge === "short" ? "Short or Long Rest" : "Long Rest")
                    font: Kirigami.Theme.smallFont
                    opacity: 0.6
                }
                Kirigami.Separator {
                    Layout.fillWidth: true
                    Layout.topMargin: Kirigami.Units.smallSpacing
                }
            }
        }

        QQC2.Label {
            visible: (App.c.features || []).length === 0
            text: "Add class features, species traits and feats here. Give a feature uses to track them."
            wrapMode: Text.WordWrap
            opacity: 0.7
            Layout.fillWidth: true
        }
    }

    Section {
        title: "Languages"
        SheetArea {
            pointer: "/languages"
            value: App.c.languages ?? ""
        }
    }

    Section {
        title: "Other Proficiencies"
        SheetArea {
            pointer: "/other_proficiencies"
            value: App.c.other_proficiencies ?? ""
            placeholderText: "Armor, weapons, tools…"
        }
    }

    ItemDialog {
        id: featureDialog
        title: "Feature"
        fields: [
            { key: "name", label: "Name", type: "text" },
            { key: "source", label: "Source", type: "text" },
            { key: "description", label: "Description", type: "area" },
            { key: "uses_max", label: "Uses (0 = untracked)", type: "int", from: 0, to: 20, visibleWhen: feature => !feature.uses_formula },
            { key: "uses_formula", label: "Uses formula", type: "text", placeholder: "e.g. {pb}" },
            { key: "recharge", label: "Recharge", type: "choice", options: App.catalog.recharges.filter(o => o.key !== "dawn") },
            { label: "Always on", type: "section" },
            { key: "modifiers", label: "Modifiers", type: "modifiers" },
            { label: "Actions", type: "section" },
            { key: "actions", label: "Actions", type: "actions", owner: "feature" }
        ]
    }
}
