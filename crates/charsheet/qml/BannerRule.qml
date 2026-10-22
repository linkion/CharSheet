import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import io.github.porave.charsheet

// The sheet's divider banner: a long tapered band on each side of a title, capped with a
// round end on the left and a ball on the right.
Item {
    id: banner

    property string text

    visible: Style.decorated
    implicitHeight: Math.max(label.implicitHeight, 18)

    readonly property real mid: height / 2
    readonly property real textLeft: label.x - 10
    readonly property real textRight: label.x + label.width + 10

    QQC2.Label {
        id: label
        anchors.centerIn: parent
        text: banner.text
        color: Style.heading
        font.family: Style.displayFamily
        font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.9
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        // Left band: a half-round cap at the far end, widening toward the title.
        ShapePath {
            strokeColor: Style.frameOuter
            strokeWidth: 1.5
            fillColor: Style.panel
            PathSvg {
                readonly property real m: banner.mid
                readonly property real end: Math.max(14, banner.textLeft)
                path: `M 10 ${m - 4} L ${end - 6} ${m - 5} L ${end} ${m} L ${end - 6} ${m + 5} L 10 ${m + 4}`
                    + ` A 6 6 0 0 1 10 ${m - 4} Z`
            }
        }
        ShapePath {
            strokeColor: Style.frameOuter
            strokeWidth: 1.5
            fillColor: "transparent"
            PathSvg {
                readonly property real m: banner.mid
                path: `M 4 ${m - 9} A 9 9 0 0 0 4 ${m + 9}`
            }
        }

        // Right band: widest at the title, narrowing to a stem that ends in a ball.
        ShapePath {
            strokeColor: Style.frameOuter
            strokeWidth: 1.5
            fillColor: Style.panel
            PathSvg {
                readonly property real m: banner.mid
                readonly property real start: Math.min(banner.width - 30, banner.textRight)
                readonly property real stem: banner.width - 24
                path: `M ${start} ${m} L ${start + 6} ${m - 5} L ${stem - 30} ${m - 4}`
                    + ` C ${stem - 10} ${m - 4} ${stem - 6} ${m - 1.5} ${stem} ${m - 1.5}`
                    + ` L ${stem} ${m + 1.5} C ${stem - 6} ${m + 1.5} ${stem - 10} ${m + 4} ${stem - 30} ${m + 4}`
                    + ` L ${start + 6} ${m + 5} Z`
            }
        }
    }

    Rectangle {
        width: 12
        height: 12
        radius: 6
        color: Style.panel
        border.color: Style.frameOuter
        border.width: 1.5
        x: banner.width - 24
        y: banner.mid - 6
    }
}
