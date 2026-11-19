import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

SheetTab {
    id: tab

    Section {
        title: "Features & Traits"
        Layout.columnSpan: tab.columns

        headerActions: [
            Kirigami.Action {
                text: "From Compendium"
                icon.name: "search"
                onTriggered: compendium.show()
            },
            Kirigami.Action {
                text: "Add Feature"
                icon.name: "list-add"
                onTriggered: featureDialog.create("/features")
            }
        ]

        Repeater {
            model: App.c.features

            SheetRow {
                id: featureRow
                required property var modelData
                required property int index
                readonly property string summary: (App.d.feature_summaries || [])[index] ?? ""
                readonly property var actions: (App.d.actions || []).filter(a => a.owner === "feature" && a.index === index)
                alternate: index % 2 === 1
                clickable: true
                onClicked: App.detailsRequested("/features", index)

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    RowLayout {
                        Layout.fillWidth: true
                        QQC2.Label {
                            text: featureRow.modelData.name
                            font.bold: true
                            Layout.fillWidth: true
                            elide: Text.ElideRight
                        }
                        QQC2.Label {
                            visible: featureRow.modelData.source.length > 0
                            text: featureRow.modelData.source
                            opacity: 0.7
                        }
                        PipRow {
                            visible: featureRow.modelData.uses_max > 0
                            total: Math.min(featureRow.modelData.uses_max, 20)
                            used: featureRow.modelData.uses_spent
                            onUsedEdited: count => App.set("/features/" + featureRow.index + "/uses_spent", count)
                        }
                        Repeater {
                            model: featureRow.actions
                            ActionButton {
                                required property var modelData
                                entry: modelData
                            }
                        }
                    }
                    Prose {
                        visible: featureRow.modelData.description.length > 0
                        source: featureRow.modelData.description
                        opacity: 0.85
                    }
                    QQC2.Label {
                        visible: featureRow.summary.length > 0
                        text: featureRow.summary
                        wrapMode: Text.WordWrap
                        color: Kirigami.Theme.positiveTextColor
                        Layout.fillWidth: true
                    }
                    QQC2.Label {
                        visible: featureRow.modelData.uses_max > 0 && featureRow.modelData.recharge !== "none"
                        text: "Recharges on a " + (featureRow.modelData.recharge === "short" ? "Short or Long Rest" : "Long Rest")
                        font: Kirigami.Theme.smallFont
                        opacity: 0.6
                    }
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

    Connections {
        target: App
        function onEditRequested(listPointer, index) {
            if (listPointer === "/features")
                featureDialog.edit(listPointer, index)
        }
    }
    CompendiumDialog {
        id: compendium
        title: "Add a Feature"
        categories: [
            { key: "feat", name: "Feats" },
            { key: "species_trait", name: "Species traits" },
            { key: "class_feature", name: "Class features" },
            { key: "subclass_feature", name: "Subclass features" },
            { key: "class_option", name: "Class options" }
        ]
    }
    ItemDialog {
        id: featureDialog
        noun: "Feature"
        fields: [
            { key: "name", label: "Name", type: "text" },
            { key: "source", label: "Source", type: "text", placeholder: "e.g. Rogue 3" },
            { label: "Description", type: "section" },
            { key: "description", label: "", type: "area" },
            { label: "Uses", type: "section", collapsedWhen: feature => !(feature.uses_max > 0) && !feature.uses_formula,
              badge: feature => feature.uses_max > 0 ? feature.uses_max + " max" : "" },
            { type: "row", fields: [
                { key: "uses_max", label: "Uses (0 = untracked)", type: "int", from: 0, to: 20, visibleWhen: feature => !feature.uses_formula },
                { key: "recharge", label: "Recharge", type: "choice", options: App.catalog.recharges.filter(o => o.key !== "dawn") }
            ] },
            { key: "uses_formula", label: "Uses formula", type: "text", placeholder: "e.g. {pb}" },
            { label: "Modifiers", type: "section", collapsedWhen: feature => !(feature.modifiers || []).length, badge: count("modifiers") },
            { label: "Always on while you have the feature.", type: "hint" },
            { key: "modifiers", label: "Modifiers", type: "modifiers" },
            { label: "Actions", type: "section", collapsedWhen: feature => !(feature.actions || []).length, badge: count("actions") },
            { key: "actions", label: "Actions", type: "actions", owner: "feature" }
        ]
    }
}
