import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Character setup: identity, classes, ability scores and defenses.
SheetTab {
    id: tab

    readonly property var armorOptions: [{ key: null, name: "No armor" }].concat(App.catalog.armor)

    Section {
        title: "Identity"

        Kirigami.FormLayout {
            Layout.fillWidth: true
            SheetField { Kirigami.FormData.label: "Name:"; pointer: "/name"; value: App.c.name ?? "" }
            SheetField { Kirigami.FormData.label: "Player:"; pointer: "/player"; value: App.c.player ?? "" }
            SheetField { Kirigami.FormData.label: "Species:"; pointer: "/species"; value: App.c.species ?? "" }
            SheetField { Kirigami.FormData.label: "Background:"; pointer: "/background"; value: App.c.background ?? "" }
            SheetField { Kirigami.FormData.label: "Alignment:"; pointer: "/alignment"; value: App.c.alignment ?? "" }
        }
    }

    Section {
        title: "Classes"

        headerActions: [
            Kirigami.Action {
                text: "Add Class"
                icon.name: "list-add"
                enabled: (App.d.total_level ?? 20) < 20
                onTriggered: App.append("/classes", { class: "wizard", level: 1 })
            }
        ]

        Repeater {
            model: App.c.classes

            ColumnLayout {
                required property var modelData
                required property int index
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                RowLayout {
                    Layout.fillWidth: true
                    QQC2.ComboBox {
                        model: App.catalog.classes
                        textRole: "name"
                        valueRole: "key"
                        currentIndex: App.catalog.classes.findIndex(o => o.key === modelData.class)
                        onActivated: App.set("/classes/" + index + "/class", currentValue)
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 9
                    }
                    SheetField {
                        pointer: "/classes/" + index + "/subclass"
                        value: modelData.subclass
                        placeholderText: "Subclass"
                        Layout.fillWidth: true
                    }
                    QQC2.Label { text: "Level" }
                    SheetSpin {
                        pointer: "/classes/" + index + "/level"
                        fieldValue: modelData.level
                        from: 1
                        to: 20
                    }
                    QQC2.ToolButton {
                        icon.name: "edit-delete"
                        enabled: App.c.classes.length > 1
                        onClicked: App.removeAt("/classes", index)
                        QQC2.ToolTip.text: "Remove class"
                        QQC2.ToolTip.visible: hovered
                    }
                }
                SheetCheck {
                    visible: modelData.class === "fighter" || modelData.class === "rogue"
                    pointer: "/classes/" + index + "/third_caster"
                    fieldValue: modelData.third_caster
                    text: "Casts spells (Eldritch Knight / Arcane Trickster)"
                }
            }
        }

        QQC2.Label {
            text: "Saving throw proficiencies come from your first class."
            font: Kirigami.Theme.smallFont
            opacity: 0.7
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }

    Section {
        title: "Ability Scores"

        GridLayout {
            columns: 4
            Layout.fillWidth: true

            QQC2.Label { text: "" }
            QQC2.Label { text: "Base"; opacity: 0.7 }
            QQC2.Label { text: "Bonus"; opacity: 0.7 }
            QQC2.Label { text: "Total"; opacity: 0.7 }

            Repeater {
                model: App.d.abilities
                delegate: QQC2.Label {
                    required property var modelData
                    required property int index
                    text: modelData.name
                    Layout.row: index + 1
                    Layout.column: 0
                    Layout.fillWidth: true
                }
            }
            Repeater {
                model: App.d.abilities
                delegate: SheetSpin {
                    required property int index
                    pointer: "/base_scores/" + index
                    fieldValue: App.c.base_scores[index]
                    from: 1
                    to: 30
                    Layout.row: index + 1
                    Layout.column: 1
                }
            }
            Repeater {
                model: App.d.abilities
                delegate: SheetSpin {
                    required property int index
                    pointer: "/score_bonuses/" + index
                    fieldValue: App.c.score_bonuses[index]
                    from: -10
                    to: 10
                    Layout.row: index + 1
                    Layout.column: 2
                }
            }
            Repeater {
                model: App.d.abilities
                delegate: QQC2.Label {
                    required property var modelData
                    required property int index
                    text: modelData.score + " (" + App.signed(modelData.modifier) + ")"
                    font.bold: true
                    Layout.row: index + 1
                    Layout.column: 3
                }
            }
        }

        QQC2.Label {
            text: "In the 2024 rules your background raises one score by 2 and another by 1, or three scores by 1. Put those, and feat bonuses, in Bonus."
            font: Kirigami.Theme.smallFont
            opacity: 0.7
            wrapMode: Text.WordWrap
            Layout.fillWidth: true
        }
    }

    Section {
        title: "Defense & Other"

        Kirigami.FormLayout {
            Layout.fillWidth: true

            QQC2.ComboBox {
                Kirigami.FormData.label: "Armor:"
                model: tab.armorOptions
                textRole: "name"
                valueRole: "key"
                currentIndex: Math.max(0, tab.armorOptions.findIndex(o => o.key === (App.c.armor ?? null)))
                onActivated: App.set("/armor", currentValue)
            }
            SheetCheck {
                text: "Shield (+2 AC)"
                pointer: "/shield"
                fieldValue: App.c.shield ?? false
            }
            SheetSpin {
                Kirigami.FormData.label: "Other AC bonus:"
                pointer: "/ac_bonus"
                fieldValue: App.c.ac_bonus ?? 0
                from: -10
                to: 20
            }
            SheetSpin {
                Kirigami.FormData.label: "Maximum HP:"
                pointer: "/hp/max"
                fieldValue: App.c.hp?.max ?? 1
                from: 1
            }
            SheetSpin {
                Kirigami.FormData.label: "Speed (ft):"
                pointer: "/speed"
                fieldValue: App.c.speed ?? 30
                stepSize: 5
                to: 300
            }
            SheetSpin {
                Kirigami.FormData.label: "Initiative bonus:"
                pointer: "/initiative_bonus"
                fieldValue: App.c.initiative_bonus ?? 0
                from: -10
                to: 20
            }
            SheetCheck {
                text: "Jack of All Trades (half proficiency)"
                pointer: "/jack_of_all_trades"
                fieldValue: App.c.jack_of_all_trades ?? false
            }
        }
    }
}
