import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// A stack of roll results that stay on screen until dismissed. It is only as large as its
// cards, so clicks around them reach the sheet.
ColumnLayout {
    id: stack

    readonly property int maxCards: 6

    visible: cards.count > 0
    spacing: Kirigami.Units.smallSpacing

    // Adds a card for each of `rolls` (StoredRoll objects, oldest first).
    function add(rolls) {
        for (const roll of rolls) {
            const mode = roll.mode !== "normal" ? " (" + roll.mode + ")" : ""
            cards.insert(0, {
                rollTotal: roll.total,
                rollLabel: roll.label,
                detail: roll.expression + mode + "  " + roll.breakdown,
                natural: roll.natural || 0,
                // As text, since a ListModel turns nested arrays into models of their own.
                rollJson: JSON.stringify(roll),
                expanded: false
            })
        }
        if (cards.count > maxCards)
            cards.remove(maxCards, cards.count - maxCards)
    }

    function clear() {
        cards.clear()
    }

    ListModel {
        id: cards
    }

    QQC2.Button {
        visible: cards.count > 1
        Layout.alignment: Qt.AlignRight
        text: "Clear all"
        icon.name: "edit-clear-all"
        onClicked: stack.clear()
    }

    ListView {
        id: list

        readonly property real available: (stack.parent ? stack.parent.height : 0)
            - Kirigami.Units.largeSpacing * 2 - Kirigami.Units.gridUnit * 3

        Layout.preferredWidth: Math.min(Kirigami.Units.gridUnit * 20,
            (stack.parent ? stack.parent.width : 0) - Kirigami.Units.largeSpacing * 2)
        Layout.preferredHeight: Math.min(contentHeight, available)
        // Newest at the bottom, in the corner.
        verticalLayoutDirection: ListView.BottomToTop
        spacing: Kirigami.Units.smallSpacing
        interactive: contentHeight > height
        clip: interactive
        model: cards

        add: Transition {
            NumberAnimation {
                property: "x"
                from: Kirigami.Units.gridUnit * 3
                to: 0
                duration: Kirigami.Units.longDuration
                easing.type: Easing.OutCubic
            }
            NumberAnimation {
                property: "opacity"
                from: 0
                to: 1
                duration: Kirigami.Units.longDuration
            }
        }
        remove: Transition {
            NumberAnimation {
                property: "opacity"
                to: 0
                duration: Kirigami.Units.shortDuration
            }
        }
        displaced: Transition {
            NumberAnimation {
                property: "y"
                duration: Kirigami.Units.shortDuration
                easing.type: Easing.OutCubic
            }
        }

        delegate: SheetCard {
            id: card

            required property int index
            required property int rollTotal
            required property string rollLabel
            required property string detail
            required property int natural
            required property string rollJson
            required property bool expanded

            readonly property real headerHeight: Kirigami.Units.gridUnit * 2.8
            readonly property string verdict: natural === 20 ? "Natural 20!" : natural === 1 ? "Natural 1" : ""
            readonly property color ink: Style.decorated ? Style.heading : Kirigami.Theme.textColor

            width: ListView.view.width
            // Explicit, since the list places a card before its layout has a size.
            height: headerHeight + topPadding + bottomPadding
                + (expanded ? breakdown.implicitHeight + Kirigami.Units.smallSpacing : 0)
            knob: "none"
            onClicked: cards.setProperty(index, "expanded", !expanded)

            contentItem: ColumnLayout {
                spacing: Kirigami.Units.smallSpacing

                RowLayout {
                    Layout.fillWidth: true
                    Layout.preferredHeight: card.headerHeight
                    spacing: Kirigami.Units.largeSpacing

                    Text {
                        Layout.minimumWidth: Kirigami.Units.gridUnit * 2.5
                        horizontalAlignment: Text.AlignHCenter
                        text: card.rollTotal
                        font.family: Style.displayFamily
                        font.pixelSize: Kirigami.Units.gridUnit * 2.2
                        color: card.natural === 20 ? Kirigami.Theme.positiveTextColor
                            : card.natural === 1 ? Kirigami.Theme.negativeTextColor : card.ink
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Text {
                            Layout.fillWidth: true
                            text: card.rollLabel + (card.verdict ? "  — " + card.verdict : "")
                            font.family: Style.labelFamily
                            font.bold: true
                            color: card.ink
                            elide: Text.ElideRight
                        }
                        Text {
                            Layout.fillWidth: true
                            text: card.detail
                            font.family: Style.uiFamily
                            color: Kirigami.Theme.disabledTextColor
                            elide: Text.ElideRight
                        }
                    }
                    Kirigami.Icon {
                        Layout.preferredWidth: Kirigami.Units.iconSizes.small
                        Layout.preferredHeight: Kirigami.Units.iconSizes.small
                        source: card.expanded ? "go-up" : "go-down"
                    }
                    QQC2.ToolButton {
                        id: dismiss
                        Layout.alignment: Qt.AlignVCenter
                        icon.name: "window-close"
                        text: "Dismiss"
                        display: QQC2.AbstractButton.IconOnly
                        onClicked: cards.remove(card.index)

                        // Its own tooltip, as the shared one would open beneath the stack.
                        QQC2.ToolTip {
                            z: stack.z + 1
                            text: dismiss.text
                            visible: dismiss.hovered
                        }
                    }
                }
                RollBreakdown {
                    id: breakdown
                    Layout.fillWidth: true
                    visible: card.expanded
                    roll: card.expanded ? JSON.parse(card.rollJson) : null
                }
            }
        }
    }
}
