import QtQuick
import io.github.linkion.charsheet

// Faint graph-paper lines, like the free-writing boxes on the sheet. Place it behind a
// text area; it only shows in the decorated themes.
Canvas {
    id: paper

    property real step: 12
    property color color: Style.faint

    visible: Style.decorated
    onWidthChanged: requestPaint()
    onHeightChanged: requestPaint()
    onColorChanged: requestPaint()

    onPaint: {
        const ctx = getContext("2d")
        ctx.reset()
        ctx.strokeStyle = color
        ctx.lineWidth = 1
        ctx.beginPath()
        for (let x = step; x < width; x += step) {
            ctx.moveTo(Math.round(x) + 0.5, 0)
            ctx.lineTo(Math.round(x) + 0.5, height)
        }
        for (let y = step; y < height; y += step) {
            ctx.moveTo(0, Math.round(y) + 0.5)
            ctx.lineTo(width, Math.round(y) + 0.5)
        }
        ctx.stroke()
    }
}
