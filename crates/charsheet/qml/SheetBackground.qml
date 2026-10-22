import QtQuick
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// A page background: the theme's stone or parchment texture, or the plain window color.
Rectangle {
    color: Kirigami.Theme.backgroundColor

    Image {
        anchors.fill: parent
        visible: Style.decorated
        source: Style.texture
        fillMode: Image.Tile
        smooth: false
    }
}
