import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

Kirigami.ApplicationWindow {
    id: root

    title: App.sheet.loaded ? App.c.name + " — CharSheet" : "CharSheet"
    minimumWidth: Kirigami.Units.gridUnit * 22
    minimumHeight: Kirigami.Units.gridUnit * 20
    width: Kirigami.Units.gridUnit * 60
    height: Kirigami.Units.gridUnit * 42

    pageStack.initialPage: LibraryPage {}
    pageStack.columnView.columnResizeMode: Kirigami.ColumnView.SingleColumn

    // Set from the command line (--open, --tab).
    property string startCharacter
    property int startTab: 0

    Component.onCompleted: {
        if (startCharacter.length === 0)
            return
        const wanted = startCharacter.toLowerCase()
        const entry = App.entries.find(e => e.id === wanted || e.name.toLowerCase() === wanted)
        if (entry) {
            App.requestedTab = startTab
            openCharacter(entry.id)
        } else {
            showPassiveNotification("No character named \"" + startCharacter + "\"", "long")
        }
    }

    function openCharacter(id) {
        if (App.sheet.load(id))
            pageStack.layers.push(Qt.resolvedUrl("SheetPage.qml"))
    }

    Connections {
        target: App
        function onNotice(message) {
            root.showPassiveNotification(message, "long")
        }
    }

    Connections {
        target: App.sheet
        function onLastErrorChanged() {
            if (App.sheet.lastError.length > 0)
                root.showPassiveNotification(App.sheet.lastError, "long")
        }
    }

    Connections {
        target: App.library
        function onLastErrorChanged() {
            if (App.library.lastError.length > 0)
                root.showPassiveNotification(App.library.lastError, "long")
        }
    }

    DiceOverlay {
        id: diceOverlay
        // Above the page and any drawer or dialog, so dice rolled from the roll log show.
        parent: QQC2.Overlay.overlay
        anchors.fill: parent
        z: 1000
        onLanded: rolls => rollCards.add(rolls)
    }

    RollCards {
        id: rollCards
        parent: QQC2.Overlay.overlay
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: Kirigami.Units.largeSpacing
        z: 1001
    }

    // Leaving the sheet takes its rolls along.
    Connections {
        target: root.pageStack.layers
        function onDepthChanged() {
            if (root.pageStack.layers.depth < 2)
                rollCards.clear()
        }
    }

    Connections {
        target: App
        function onThrown(rolls) {
            if (App.appearance.dice3d)
                diceOverlay.show(rolls)
            else
                rollCards.add(rolls)
        }
    }

    RollLogDrawer {
        id: rollLog
    }

    function showRollLog() {
        rollLog.open()
    }
}
