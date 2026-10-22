import QtQuick
import io.github.porave.charsheet

// The numbers of one kind of die, a grid cell per face, for a die's texture. `mesh` is
// DiceTable.mesh(): each face's UVs cover its cell, and `cells` holds the labels to draw.
Rectangle {
    id: atlas

    required property var mesh
    property color ink: "white"
    readonly property int cell: 128

    width: mesh.columns * cell
    height: mesh.rows * cell

    Repeater {
        model: atlas.mesh.cells

        Item {
            id: face
            required property var modelData
            required property int index
            x: (index % atlas.mesh.columns) * atlas.cell
            y: Math.floor(index / atlas.mesh.columns) * atlas.cell
            width: atlas.cell
            height: atlas.cell

            Repeater {
                model: face.modelData

                Text {
                    required property var modelData
                    text: modelData.text
                    color: atlas.ink
                    font.family: Style.labelFamily
                    font.bold: true
                    font.underline: modelData.underline
                    font.pixelSize: modelData.size * atlas.cell
                    x: modelData.x * atlas.cell - width / 2
                    y: modelData.y * atlas.cell - height / 2
                    rotation: modelData.angle
                }
            }
        }
    }
}
