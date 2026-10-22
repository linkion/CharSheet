import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// Free-form dice entry plus quick buttons for the common dice.
ColumnLayout {
    RowLayout {
        Layout.fillWidth: true
        QQC2.TextField {
            id: expression
            Layout.fillWidth: true
            placeholderText: "e.g. 2d6+3, 4d6kh3"
            onAccepted: rollButton.clicked()
        }
        QQC2.Button {
            id: rollButton
            text: "Roll"
            icon.name: "roll"
            enabled: expression.text.trim().length > 0
            onClicked: App.roll(expression.text, expression.text, App.rollMode)
        }
    }
    Flow {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing
        Repeater {
            model: [4, 6, 8, 10, 12, 20, 100]
            QQC2.Button {
                required property int modelData
                text: "d" + modelData
                flat: true
                onClicked: App.roll("1d" + modelData, "d" + modelData, modelData === 20 ? App.rollMode : "normal")
            }
        }
    }
}
