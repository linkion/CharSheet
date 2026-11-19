import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Shows a signed modifier; clicking rolls `target`, a play::RollTarget, which adds it to a d20.
QQC2.Button {
    id: button

    property int modifier
    property var target
    property string rollLabel
    // Text size relative to the default font.
    property real textScale: 1
    // Where the modifier comes from, as bonuses from the derived view; when set, hovering
    // shows them instead of a plain "Roll …" tooltip.
    property var parts
    // Set by a label beside the button to show the breakdown while it's hovered too.
    property bool showBreakdown: false

    // The desktop style paints `text` itself, under the label below, so it stays empty.
    readonly property string label: App.signed(modifier)

    flat: true
    Accessible.name: label
    onClicked: App.rollCheck(target)
    QQC2.ToolTip.text: "Roll " + rollLabel
    QQC2.ToolTip.visible: hovered && !parts
    QQC2.ToolTip.delay: 600

    HoverHandler {
        id: hover
    }
    BreakdownTip {
        visible: !!button.parts && (hover.hovered || button.showBreakdown)
        parts: button.parts ?? []
        total: button.modifier
        mode: "modifier"
        title: button.rollLabel
        hint: "Click to roll"
    }

    // The desktop style ignores font changes on buttons, so draw the label ourselves.
    contentItem: QQC2.Label {
        text: button.label
        font.bold: true
        // Large modifiers use the sheet's display face; row-sized ones stay in the UI font.
        font.family: button.textScale > 1 ? Style.displayFamily : Style.uiFamily
        font.pointSize: Kirigami.Theme.defaultFont.pointSize * button.textScale
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
