import QtQuick
import org.kde.kirigami as Kirigami
import io.github.linkion.charsheet

// A card that is drawn as a character sheet box in the decorated themes, and as a plain
// Kirigami card in the System theme.
Kirigami.AbstractCard {
    id: card

    property alias knob: frame.knob
    property alias curl: frame.curl
    // False when the card draws its own outline, such as a shield.
    property bool framed: true

    padding: Style.decorated ? Kirigami.Units.largeSpacing + 4 : Kirigami.Units.largeSpacing
    background.visible: !Style.decorated

    SheetFrame {
        id: frame
        parent: card
        z: -1
        anchors.fill: parent
        visible: Style.decorated && card.framed
    }
}
