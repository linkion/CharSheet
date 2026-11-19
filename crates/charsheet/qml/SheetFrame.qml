import QtQuick
import QtQuick.Shapes
import io.github.linkion.charsheet

// The character sheet's box outline: a heavy line with a hairline just inside it, the top
// corners scooped out (one of them holding a small knob) and the bottom corners cut off.
Item {
    id: frame

    // Which top corner gets the knob: "left", "right" or "none".
    property string knob: "left"
    // Radius of the scooped top corners and size of the cut bottom corners.
    property real curl: 12
    property real chamfer: 7
    // Distance between the outer line and the hairline.
    property real gap: 3.5
    property color fill: Style.panel
    property color lineColor: Style.frameOuter
    property color innerColor: Style.frameInner

    // The outline inset by `d`. The scoops are arcs around the box's own corners, so every
    // inset of them stays parallel.
    function outline(d) {
        const w = width, h = height, r = curl, c = chamfer
        const a = Math.sqrt(Math.max(0, r * r - d * d))
        const k = d * (Math.SQRT2 - 1)
        return `M ${a} ${d} L ${w - a} ${d} A ${r} ${r} 0 0 0 ${w - d} ${a}`
            + ` L ${w - d} ${h - c - k} L ${w - c - k} ${h - d} L ${c + k} ${h - d} L ${d} ${h - c - k}`
            + ` L ${d} ${a} A ${r} ${r} 0 0 0 ${a} ${d} Z`
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: frame.lineColor
            strokeWidth: 2
            fillColor: frame.fill
            joinStyle: ShapePath.MiterJoin
            PathSvg { path: frame.outline(1) }
        }
        ShapePath {
            strokeColor: frame.innerColor
            strokeWidth: 1
            fillColor: "transparent"
            PathSvg { path: frame.outline(1 + frame.gap) }
        }
    }

    Rectangle {
        visible: frame.knob !== "none"
        width: 9
        height: 9
        radius: width / 2
        color: frame.lineColor
        x: frame.knob === "left" ? -2 : frame.width - width + 2
        y: -2
    }
}
