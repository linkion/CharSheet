import QtQuick
import QtQuick3D
import QtQuick3D.Helpers

// One thrown die, played back from its simulated frames (DiceTable.throwRoll).
Node {
    id: die

    // A die of the throw: shape, kept, offset and frames.
    required property var thrown
    required property int fps
    // Seconds since the throw.
    required property real time
    required property var geometry
    required property Texture atlas

    readonly property real step: Math.min(time * fps, thrown.frames.length - 1)

    function blend(a, b, t) {
        // Takes the short way round between the two rotations.
        const s = a[3] * b[3] + a[4] * b[4] + a[5] * b[5] + a[6] * b[6] < 0 ? -1 : 1
        const q = [3, 4, 5, 6].map(i => a[i] * (1 - t) + s * b[i] * t)
        const n = Math.hypot(q[0], q[1], q[2], q[3])
        return {
            position: Qt.vector3d(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t),
            rotation: Qt.quaternion(q[3] / n, q[0] / n, q[1] / n, q[2] / n)
        }
    }

    readonly property var pose: {
        const i = Math.floor(step)
        const frames = thrown.frames
        return blend(frames[i], frames[Math.min(i + 1, frames.length - 1)], step - i)
    }

    position: pose.position
    rotation: pose.rotation

    Model {
        // Turns the right number onto the face that lands up.
        rotation: Qt.quaternion(die.thrown.offset[3], die.thrown.offset[0], die.thrown.offset[1], die.thrown.offset[2])
        geometry: ProceduralMesh {
            positions: die.geometry.positions
            normals: die.geometry.normals
            uv0s: die.geometry.uvs
            indexes: die.geometry.indexes
        }
        materials: PrincipledMaterial {
            baseColorMap: die.atlas
            roughness: 0.35
            metalness: 0.0
            specularAmount: 0.6
            // Dice the roll dropped, like the low d20 at advantage, are see-through.
            opacity: die.thrown.kept ? 1 : 0.35
            alphaMode: die.thrown.kept ? PrincipledMaterial.Opaque : PrincipledMaterial.Blend
        }
    }
}
