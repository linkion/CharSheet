import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Shapes
import org.kde.kirigami as Kirigami

// A four-pointed diamond check box, like the sheet's death save and spell slot marks.
QQC2.AbstractButton {
    id: pip

    property real size: Kirigami.Units.gridUnit * 1.1

    checkable: true
    implicitWidth: size
    implicitHeight: size
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    // A diamond with slightly pinched sides, inset by `d` within the s x s box.
    function diamond(s, d) {
        const c = s / 2, t = d, b = s - d, k = (s - 2 * d) * 0.1
        return `M ${c} ${t} Q ${c + k} ${c - k} ${b} ${c} Q ${c + k} ${c + k} ${c} ${b}`
            + ` Q ${c - k} ${c + k} ${t} ${c} Q ${c - k} ${c - k} ${c} ${t} Z`
    }

    contentItem: Shape {
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: pip.hovered || pip.visualFocus ? Kirigami.Theme.focusColor : Kirigami.Theme.textColor
            strokeWidth: 1.6
            fillColor: pip.checked ? Kirigami.Theme.textColor : "transparent"
            joinStyle: ShapePath.MiterJoin
            PathSvg { path: pip.diamond(pip.size, 1.5) }
        }
    }
}
