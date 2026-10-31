import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Where a roll's total came from: a line per term with its source, and for damage a heading
// and subtotal per type. Free-form dice and rolls logged before sources were kept have only
// their dice to show.
ColumnLayout {
    id: breakdown

    // A StoredRoll.
    property var roll

    // Terms in the order they were rolled, grouped by damage type in the order types appear.
    readonly property var groups: {
        const found = []
        for (const term of roll?.terms ?? []) {
            const type = term.damage_type ?? ""
            let group = found.find(g => g.type === type)
            if (!group) {
                group = { type: type, total: 0, terms: [] }
                found.push(group)
            }
            group.total += term.sign * term.value
            group.terms.push(term)
        }
        return found
    }
    readonly property bool typed: groups.some(g => g.type !== "")

    function dice(term) {
        if (term.rolls.length === 0)
            return App.signed(term.sign * term.value)
        const shown = term.rolls.map(r => r[1] ? r[0] : "<s>" + r[0] + "</s>").join(", ")
        return (term.sign < 0 ? "-" : "") + "[" + shown + "]"
    }

    spacing: 0

    Repeater {
        model: breakdown.groups

        ColumnLayout {
            id: group

            required property var modelData

            Layout.fillWidth: true
            spacing: 0

            RowLayout {
                visible: breakdown.typed
                Layout.fillWidth: true

                QQC2.Label {
                    Layout.fillWidth: true
                    text: group.modelData.type ? App.capitalized(group.modelData.type) : "Untyped"
                    font.bold: true
                }
                QQC2.Label {
                    text: group.modelData.total
                    font.bold: true
                }
            }

            Repeater {
                model: group.modelData.terms

                RowLayout {
                    required property var modelData

                    Layout.fillWidth: true
                    Layout.leftMargin: breakdown.typed ? Kirigami.Units.largeSpacing : 0
                    spacing: Kirigami.Units.largeSpacing

                    QQC2.Label {
                        Layout.minimumWidth: Kirigami.Units.gridUnit * 3
                        text: breakdown.dice(modelData)
                        textFormat: Text.StyledText
                    }
                    QQC2.Label {
                        Layout.fillWidth: true
                        text: modelData.source || modelData.label
                        elide: Text.ElideRight
                        opacity: 0.7
                    }
                    QQC2.Label {
                        text: modelData.sign * modelData.value
                    }
                }
            }
        }
    }
}
