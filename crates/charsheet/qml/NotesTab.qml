import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

SheetTab {
    wideColumns: 1

    Section {
        title: "Appearance & Personality"
        SheetArea {
            pointer: "/appearance"
            value: App.c.appearance ?? ""
        }
    }

    Section {
        title: "Backstory"
        SheetArea {
            pointer: "/backstory"
            value: App.c.backstory ?? ""
            Layout.minimumHeight: Kirigami.Units.gridUnit * 6
        }
    }

    Section {
        title: "Notes"
        SheetArea {
            pointer: "/notes"
            value: App.c.notes ?? ""
            Layout.minimumHeight: Kirigami.Units.gridUnit * 8
        }
    }
}
