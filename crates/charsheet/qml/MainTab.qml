import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

SheetTab {
    id: tab

    readonly property var proficiencyGlyph: ({ none: "○", proficient: "●", expertise: "◉" })
    readonly property var nextProficiency: ({ none: "proficient", proficient: "expertise", expertise: "none" })

    // Summary line and quick stats span the full width.
    QQC2.Label {
        Layout.fillWidth: true
        Layout.columnSpan: 2
        text: "Level " + App.d.total_level + " " + [App.c.species, App.d.class_summary].filter(s => s).join(" · ")
              + (App.c.background ? "  —  " + App.c.background : "")
        wrapMode: Text.WordWrap
        opacity: 0.8
    }

    GridLayout {
        Layout.fillWidth: true
        Layout.columnSpan: 2
        columns: width > Kirigami.Units.gridUnit * 36 ? 6 : 3
        columnSpacing: Kirigami.Units.largeSpacing
        rowSpacing: Kirigami.Units.largeSpacing

        Repeater {
            model: App.d.abilities

            Kirigami.AbstractCard {
                required property var modelData
                Layout.fillWidth: true
                contentItem: ColumnLayout {
                    spacing: 0
                    QQC2.Label {
                        text: modelData.name
                        font.bold: true
                        horizontalAlignment: Text.AlignHCenter
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                    RollButton {
                        modifier: modelData.modifier
                        rollLabel: modelData.name + " check"
                        textScale: 1.8
                        Layout.alignment: Qt.AlignHCenter
                    }
                    QQC2.Label {
                        text: modelData.score
                        opacity: 0.7
                        horizontalAlignment: Text.AlignHCenter
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }

    GridLayout {
        Layout.fillWidth: true
        Layout.columnSpan: 2
        columns: width > Kirigami.Units.gridUnit * 36 ? 7 : 4
        columnSpacing: Kirigami.Units.largeSpacing
        rowSpacing: Kirigami.Units.largeSpacing

        StatTile { label: "Proficiency"; value: App.signed(App.d.proficiency_bonus ?? 0) }
        StatTile { label: "Armor Class"; value: App.d.armor_class ?? "" }
        StatTile { label: "Initiative"; rollable: true; rollModifier: App.d.initiative ?? 0 }
        StatTile { label: "Speed"; value: (App.d.speed ?? 0) + " ft" }
        StatTile { label: "Passive Perception"; value: App.d.passive_perception ?? "" }
        StatTile { label: "Passive Insight"; value: App.d.passive_insight ?? "" }
        Kirigami.AbstractCard {
            Layout.fillWidth: true
            contentItem: ColumnLayout {
                QQC2.Label {
                    text: "Heroic Inspiration"
                    opacity: 0.7
                    font: Kirigami.Theme.smallFont
                    horizontalAlignment: Text.AlignHCenter
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }
                QQC2.Switch {
                    checked: App.c.heroic_inspiration ?? false
                    onToggled: App.set("/heroic_inspiration", checked)
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        Layout.columnSpan: 2
        visible: (App.c.exhaustion ?? 0) > 0
        type: Kirigami.MessageType.Warning
        text: "Exhaustion " + App.c.exhaustion + ": " + App.d.d20_penalty + " to every d20 test (already included below)."
    }

    Section {
        title: "Saving Throws"

        Repeater {
            model: App.d.abilities

            RowLayout {
                required property var modelData
                Layout.fillWidth: true

                QQC2.CheckBox {
                    checked: modelData.save_proficient
                    enabled: !modelData.save_from_class
                    onToggled: {
                        const saves = (App.c.extra_saves || []).filter(a => a !== modelData.key)
                        if (checked)
                            saves.push(modelData.key)
                        App.set("/extra_saves", saves)
                    }
                    QQC2.ToolTip.text: modelData.save_from_class ? "Proficient from your starting class" : "Proficient"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.Label {
                    text: modelData.name
                    Layout.fillWidth: true
                }
                RollButton {
                    modifier: modelData.save
                    rollLabel: modelData.name + " save"
                }
            }
        }

        Kirigami.Separator { Layout.fillWidth: true }

        RowLayout {
            Layout.fillWidth: true
            QQC2.Label {
                text: "Passive Investigation"
                Layout.fillWidth: true
            }
            QQC2.Label {
                text: App.d.passive_investigation ?? ""
                font.bold: true
                rightPadding: Kirigami.Units.largeSpacing
            }
        }
    }

    Section {
        title: "Skills"

        Repeater {
            model: App.d.skills

            RowLayout {
                required property var modelData
                Layout.fillWidth: true

                QQC2.ToolButton {
                    text: tab.proficiencyGlyph[modelData.proficiency]
                    onClicked: App.set("/skills/" + modelData.key, tab.nextProficiency[modelData.proficiency])
                    QQC2.ToolTip.text: ({ none: "Not proficient", proficient: "Proficient", expertise: "Expertise" })[modelData.proficiency] + " — click to change"
                    QQC2.ToolTip.visible: hovered
                }
                QQC2.Label {
                    text: modelData.name
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }
                QQC2.Label {
                    text: modelData.ability
                    opacity: 0.6
                    font: Kirigami.Theme.smallFont
                }
                RollButton {
                    modifier: modelData.modifier
                    rollLabel: modelData.name
                }
            }
        }
    }
}
