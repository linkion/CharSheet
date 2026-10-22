import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// Scrollable tab content laid out as a responsive grid of sections.
QQC2.ScrollView {
    id: tab

    property int wideColumns: 2
    default property alias content: grid.data

    contentWidth: availableWidth

    Item {
        width: tab.availableWidth
        implicitHeight: grid.implicitHeight + Kirigami.Units.largeSpacing * 2

        GridLayout {
            id: grid
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
                margins: Kirigami.Units.largeSpacing
            }
            columns: width > Kirigami.Units.gridUnit * 36 ? tab.wideColumns : 1
            columnSpacing: Kirigami.Units.largeSpacing
            rowSpacing: Kirigami.Units.largeSpacing
        }
    }
}
