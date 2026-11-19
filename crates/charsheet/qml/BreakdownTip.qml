import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// A tooltip showing where a number comes from: a line per source, then the total.
QQC2.ToolTip {
    id: tip

    // Bonuses from the derived view, each { value, source }.
    property var parts: []
    // The number shown on the sheet; it can differ from the sum when kept above a floor.
    property var total
    property string title
    property string hint
    // "modifier": every number signed (+3). "value": a base number then signed additions
    // (AC 12, DEX +3). "sum": plain numbers that add up (class levels).
    property string mode: "value"
    // Appended to the total, e.g. " ft".
    property string unit

    // Zero bonuses add nothing, so only the first (the base, like CHA +0) is kept.
    readonly property var shown: (parts || []).filter((p, i) => i === 0 || p.value !== 0)

    function format(value, index) {
        if (mode === "modifier" || (mode === "value" && index > 0))
            return App.signed(value)
        return String(value)
    }

    delay: 400
    contentItem: GridLayout {
        columns: 2
        columnSpacing: Kirigami.Units.largeSpacing * 2
        rowSpacing: 2

        QQC2.Label {
            visible: tip.title !== ""
            text: tip.title
            font.bold: true
            Layout.row: 0
            Layout.column: 0
            Layout.columnSpan: 2
        }

        Repeater {
            model: tip.shown

            QQC2.Label {
                required property var modelData
                required property int index
                // Each part fills two cells: its source, then its value.
                text: modelData.source
                opacity: 0.75
                Layout.row: index + 1
                Layout.column: 0
            }
        }
        Repeater {
            model: tip.shown

            QQC2.Label {
                required property var modelData
                required property int index
                text: tip.format(modelData.value, index)
                horizontalAlignment: Text.AlignRight
                Layout.row: index + 1
                Layout.column: 1
                Layout.fillWidth: true
            }
        }

        Rectangle {
            visible: tip.shown.length > 1
            implicitHeight: 1
            color: Kirigami.Theme.textColor
            opacity: 0.3
            Layout.row: tip.shown.length + 1
            Layout.columnSpan: 2
            Layout.fillWidth: true
        }
        QQC2.Label {
            visible: tip.shown.length > 1
            text: "Total"
            font.bold: true
            Layout.row: tip.shown.length + 2
            Layout.column: 0
        }
        QQC2.Label {
            visible: tip.shown.length > 1
            text: (tip.mode === "modifier" ? App.signed(tip.total) : tip.total) + tip.unit
            font.bold: true
            horizontalAlignment: Text.AlignRight
            Layout.row: tip.shown.length + 2
            Layout.column: 1
            Layout.fillWidth: true
        }

        QQC2.Label {
            visible: tip.hint !== ""
            text: tip.hint
            opacity: 0.6
            font: Kirigami.Theme.smallFont
            Layout.row: tip.shown.length + 3
            Layout.columnSpan: 2
        }
    }
}
