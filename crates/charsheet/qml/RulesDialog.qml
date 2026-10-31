import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// The rules text of an SRD record. Links inside it open here too, with Back to return.
Kirigami.Dialog {
    id: dialog

    property var history: []
    property var entry: ({})

    function show(id) {
        const json = Srd.record(id)
        if (!json) {
            App.report("The rules have no entry for " + id)
            return
        }
        if (opened && entry.id && entry.id !== id)
            history = history.concat([entry.id])
        else if (!opened)
            history = []
        entry = JSON.parse(json)
        open()
    }

    function back() {
        entry = JSON.parse(Srd.record(history[history.length - 1]))
        history = history.slice(0, -1)
    }

    title: entry.name ?? ""
    standardButtons: Kirigami.Dialog.Close
    preferredWidth: Kirigami.Units.gridUnit * 30
    padding: Kirigami.Units.largeSpacing

    customFooterActions: [
        Kirigami.Action {
            text: "Back"
            icon.name: "go-previous"
            enabled: dialog.history.length > 0
            onTriggered: dialog.back()
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Prose {
            source: dialog.entry.markdown ?? ""
        }
        ProseLine {
            visible: dialog.entry.id !== "about"
            links: "<a href=\"srd:about\">SRD 5.2.1 · CC-BY-4.0</a>" + (dialog.entry.page ? " · page " + dialog.entry.page : "")
            font: Kirigami.Theme.smallFont
            opacity: 0.7
        }
    }
}
