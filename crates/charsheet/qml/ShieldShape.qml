import QtQuick
import QtQuick.Shapes
import io.github.linkion.charsheet

// The Armor Class shield outline: notched top corners, straight sides and a pointed base,
// drawn double like the sheet's boxes.
Item {
    id: shield

    property color fill: Style.panel

    // The outline inset by `d`, in a box of w x h.
    function outline(d) {
        const w = width - 2 * d, h = height - 2 * d, n = Math.min(w, h) * 0.09
        const p = (x, y) => `${d + x} ${d + y}`
        return `M ${p(n, 0)} L ${p(w - n, 0)} L ${p(w, n)} L ${p(w, h * 0.55)}`
            + ` C ${p(w, h * 0.78)} ${p(w * 0.72, h * 0.9)} ${p(w / 2, h)}`
            + ` C ${p(w * 0.28, h * 0.9)} ${p(0, h * 0.78)} ${p(0, h * 0.55)}`
            + ` L ${p(0, n)} Z`
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: Style.frameOuter
            strokeWidth: 2
            fillColor: shield.fill
            PathSvg { path: shield.outline(1) }
        }
        ShapePath {
            strokeColor: Style.frameInner
            strokeWidth: 1
            fillColor: "transparent"
            PathSvg { path: shield.outline(5) }
        }
    }
}
