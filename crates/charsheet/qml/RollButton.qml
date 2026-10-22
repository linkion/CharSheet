import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Shows a signed modifier; clicking rolls a d20 with it.
QQC2.Button {
    id: button

    property int modifier
    property string rollLabel
    // Text size relative to the default font.
    property real textScale: 1

    text: App.signed(modifier)
    flat: true
    onClicked: App.rollD20(modifier, rollLabel)
    QQC2.ToolTip.text: "Roll " + rollLabel
    QQC2.ToolTip.visible: hovered
    QQC2.ToolTip.delay: 600

    // The desktop style ignores font changes on buttons, so draw the label ourselves.
    contentItem: QQC2.Label {
        text: button.text
        font.bold: true
        font.pointSize: Kirigami.Theme.defaultFont.pointSize * button.textScale
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
