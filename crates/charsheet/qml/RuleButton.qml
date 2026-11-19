import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// Opens the rules for a sheet entry, showing only when the SRD has an entry of that name.
QQC2.ToolButton {
    id: button

    required property string name
    property var categories: ["spell"]
    readonly property string record: Srd.find(name, JSON.stringify(categories))

    visible: record.length > 0
    icon.name: "help-about"
    display: QQC2.AbstractButton.IconOnly
    text: "Rules for " + name
    onClicked: App.ruleRequested(record)
    QQC2.ToolTip.text: text
    QQC2.ToolTip.visible: hovered
    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
}
