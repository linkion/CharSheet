import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// The front of the character sheet: who the character is, their headline numbers, and
// each ability with its saving throw and skills.
SheetTab {
    id: tab

    readonly property bool wide: width > Kirigami.Units.gridUnit * 36

    function skillsFor(ability) {
        return (App.d.skills || []).filter(s => s.ability === ability.abbrev)
    }

    // Identity, level, armor class and hit points, like the top band of the sheet.
    GridLayout {
        Layout.fillWidth: true
        Layout.columnSpan: tab.columns
        columns: tab.wide ? 4 : 3
        columnSpacing: Kirigami.Units.largeSpacing
        rowSpacing: Kirigami.Units.largeSpacing

        SheetCard {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.columnSpan: tab.wide ? 1 : 3
            Layout.preferredWidth: Kirigami.Units.gridUnit * 20

            contentItem: GridLayout {
                columns: 2
                columnSpacing: Kirigami.Units.largeSpacing
                rowSpacing: Kirigami.Units.smallSpacing

                Repeater {
                    model: [
                        ["Background", App.c.background],
                        ["Class", App.d.class_summary],
                        ["Species", App.c.species],
                        ["Alignment", App.c.alignment]
                    ]

                    ColumnLayout {
                        required property var modelData
                        spacing: 1
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1

                        QQC2.Label {
                            text: modelData[1] || "—"
                            elide: Text.ElideRight
                            Layout.fillWidth: true
                        }
                        Rectangle {
                            implicitHeight: 1
                            color: Style.decorated ? Style.frameInner : Kirigami.Theme.disabledTextColor
                            Layout.fillWidth: true
                        }
                        QQC2.Label {
                            text: modelData[0]
                            opacity: 0.75
                            font.family: Style.labelFamily
                            font.capitalization: Font.AllUppercase
                            font.letterSpacing: 1
                            font.pointSize: Kirigami.Theme.smallFont.pointSize
                        }
                    }
                }
            }
        }

        StatTile {
            label: "Level"
            value: App.d.total_level ?? ""
            parts: App.d.level_parts
            partsMode: "sum"
            shape: "oval"
            Layout.fillWidth: false
            Layout.preferredWidth: Kirigami.Units.gridUnit * 6
            Layout.fillHeight: true
        }
        StatTile {
            label: "Armor Class"
            value: App.d.armor_class ?? ""
            parts: App.d.armor_class_parts
            shape: "shield"
            Layout.fillWidth: false
            Layout.preferredWidth: Kirigami.Units.gridUnit * 6.5
            Layout.fillHeight: true
        }
        StatTile {
            label: "Hit Points"
            value: (App.c.hp?.current ?? 0) + " / " + (App.d.max_hp ?? 0)
                   + ((App.c.hp?.temp ?? 0) > 0 ? "  +" + App.c.hp.temp : "")
            parts: (App.d.max_hp_parts ?? []).concat(
                       (App.c.hp?.temp ?? 0) > 0 ? [{ value: App.c.hp.temp, source: "Temporary" }] : [])
            partsTotal: (App.d.max_hp ?? 0) + (App.c.hp?.temp ?? 0)
            Layout.preferredWidth: Kirigami.Units.gridUnit * 9
            Layout.fillHeight: true
        }
    }

    BannerRule {
        Layout.fillWidth: true
        Layout.columnSpan: tab.columns
        text: App.c.name || ""
    }

    GridLayout {
        Layout.fillWidth: true
        Layout.columnSpan: tab.columns
        columns: tab.wide ? 7 : 4
        columnSpacing: Kirigami.Units.largeSpacing
        rowSpacing: Kirigami.Units.largeSpacing

        StatTile {
            label: "Proficiency"
            value: App.signed(App.d.proficiency_bonus ?? 0)
            parts: App.d.proficiency_parts
            partsMode: "modifier"
            partsTotal: App.d.proficiency_bonus ?? 0
        }
        StatTile {
            label: "Initiative"
            rollable: true
            rollModifier: App.d.initiative ?? 0
            rollTarget: ({ kind: "initiative" })
            parts: App.d.initiative_parts
        }
        StatTile {
            label: "Speed"
            value: (App.d.speed ?? 0) + " ft"
            parts: App.d.speed_parts
            partsTotal: App.d.speed ?? 0
            partsUnit: " ft"
        }
        StatTile { label: "Passive Perception"; value: App.d.passive_perception ?? ""; parts: App.d.passive_perception_parts }
        StatTile { label: "Passive Insight"; value: App.d.passive_insight ?? ""; parts: App.d.passive_insight_parts }
        StatTile { label: "Passive Investigation"; value: App.d.passive_investigation ?? ""; parts: App.d.passive_investigation_parts }
        SheetCard {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumWidth: Kirigami.Units.gridUnit * 5
            Layout.preferredWidth: Kirigami.Units.gridUnit * 6
            contentItem: ColumnLayout {
                FrameTitle {
                    visible: Style.decorated
                    text: "Heroic Inspiration"
                    scale: 1
                }
                QQC2.Label {
                    visible: !Style.decorated
                    text: "Heroic Inspiration"
                    opacity: 0.7
                    font: Kirigami.Theme.smallFont
                    horizontalAlignment: Text.AlignHCenter
                    Layout.fillWidth: true
                    elide: Text.ElideRight
                }
                DiamondPip {
                    visible: Style.decorated
                    size: Kirigami.Units.gridUnit * 1.8
                    checked: App.c.heroic_inspiration ?? false
                    onToggled: App.set("/heroic_inspiration", checked)
                    Layout.alignment: Qt.AlignHCenter
                }
                QQC2.Switch {
                    visible: !Style.decorated
                    checked: App.c.heroic_inspiration ?? false
                    onToggled: App.set("/heroic_inspiration", checked)
                    Layout.alignment: Qt.AlignHCenter
                }
            }
        }
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        Layout.columnSpan: tab.columns
        visible: (App.c.exhaustion ?? 0) > 0
        type: Kirigami.MessageType.Warning
        text: "Exhaustion " + App.c.exhaustion + ": " + App.d.d20_penalty + " to every d20 test (already included below)."
    }

    // The abilities in two columns, as on the sheet: STR, DEX, CON, then INT, WIS, CHA.
    Repeater {
        model: [[0, 1, 2], [3, 4, 5]]

        ColumnLayout {
            required property var modelData
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: 1
            spacing: Kirigami.Units.largeSpacing

            Repeater {
                model: modelData.map(i => (App.d.abilities || [])[i]).filter(a => a)

                AbilityBox {
                    required property var modelData
                    ability: modelData
                    skills: tab.skillsFor(modelData)
                }
            }
        }
    }
}
