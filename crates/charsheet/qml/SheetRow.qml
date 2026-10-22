import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// A row of a list whose buttons sit at the far end: striped every other row and lit up
// under the pointer, so it's clear which entry a button belongs to.
QQC2.Control {
    id: row

    // Every other row gets the stripe.
    property bool alternate
    default property alias content: layout.data

    Layout.fillWidth: true
    horizontalPadding: Kirigami.Units.smallSpacing
    verticalPadding: Kirigami.Units.smallSpacing / 2

    HoverHandler { id: hover }

    background: Rectangle {
        radius: Kirigami.Units.cornerRadius
        color: hover.hovered ? Qt.alpha(Kirigami.Theme.highlightColor, 0.15)
             : !row.alternate ? "transparent"
             : Style.decorated ? Style.faint
             : Kirigami.Theme.alternateBackgroundColor
    }

    contentItem: RowLayout {
        id: layout
    }
}
