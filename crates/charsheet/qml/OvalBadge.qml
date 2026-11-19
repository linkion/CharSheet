import QtQuick
import QtQuick.Shapes
import io.github.linkion.charsheet

// The upright oval that holds the character's level, with a small notch at the top.
Item {
    id: oval

    property color fill: Style.panel

    function ellipse(d) {
        const rx = width / 2 - d, ry = height / 2 - d, cx = width / 2
        return `M ${cx} ${d} A ${rx} ${ry} 0 1 1 ${cx} ${height - d} A ${rx} ${ry} 0 1 1 ${cx} ${d} Z`
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: Style.frameOuter
            strokeWidth: 2.5
            fillColor: oval.fill
            PathSvg { path: oval.ellipse(1.25) }
        }
        ShapePath {
            strokeColor: Style.frameInner
            strokeWidth: 1
            fillColor: "transparent"
            PathSvg { path: oval.ellipse(5) }
        }
        ShapePath {
            strokeColor: Style.frameOuter
            strokeWidth: 1.5
            fillColor: "transparent"
            PathSvg {
                readonly property real cx: oval.width / 2
                path: `M ${cx - 6} 5 L ${cx} 12 L ${cx + 6} 5`
            }
        }
    }
}
