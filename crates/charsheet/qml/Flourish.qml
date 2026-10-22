import QtQuick
import QtQuick.Shapes
import io.github.porave.charsheet

// Curving ribbon lines down both sides of a stat box, like the curtain-shaped strokes
// behind the numbers on the character sheet. Purely decorative.
Shape {
    id: flourish

    property color color: Style.frameInner

    preferredRendererType: Shape.CurveRenderer
    visible: Style.decorated

    // Two strands that cross once, on the side from x = 0 to x = s (mirrored when s < 0).
    function strands(x0, s) {
        const h = height
        return `M ${x0} 0 C ${x0 + s * 0.9} ${h * 0.3} ${x0 + s * 0.1} ${h * 0.7} ${x0 + s} ${h}`
            + ` M ${x0 + s * 0.75} 0 C ${x0 - s * 0.1} ${h * 0.35} ${x0 + s * 1.1} ${h * 0.6} ${x0 + s * 0.45} ${h}`
    }

    ShapePath {
        strokeColor: flourish.color
        strokeWidth: 1.2
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap
        PathSvg {
            readonly property real s: Math.min(flourish.width * 0.16, 22)
            path: flourish.strands(4, s) + " " + flourish.strands(flourish.width - 4, -s)
        }
    }
}
