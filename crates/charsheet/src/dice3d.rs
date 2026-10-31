//! 3D dice: each die's mesh, and a physics throw that lands on numbers already rolled.
//!
//! The whole throw is simulated up front, then played back by DiceOverlay.qml. A die solid
//! looks the same after any of its symmetry rotations, so once the simulation shows which
//! face ends up on top, the mesh is turned inside its body by the symmetry that brings the
//! wanted number to that face. The collider is unchanged, so the physics is too.

use dnd_core::dice::RollResult;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rapier3d::glamx::{Mat3, Quat, Vec3};
use rapier3d::prelude::*;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    D4,
    D6,
    D8,
    D10,
    D12,
    D20,
    /// The tens die of a d100: a d10 marked 00 to 90.
    D100,
}

impl Shape {
    pub const ALL: [Shape; 7] = [
        Shape::D4,
        Shape::D6,
        Shape::D8,
        Shape::D10,
        Shape::D12,
        Shape::D20,
        Shape::D100,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Shape::D4 => "d4",
            Shape::D6 => "d6",
            Shape::D8 => "d8",
            Shape::D10 => "d10",
            Shape::D12 => "d12",
            Shape::D20 => "d20",
            Shape::D100 => "d100",
        }
    }

    pub fn from_name(name: &str) -> Option<Shape> {
        Shape::ALL.into_iter().find(|s| s.name() == name)
    }

    /// Number of faces, and of values: a d10's 10 is marked 0, a d100's 10 is marked 00.
    pub fn sides(self) -> u32 {
        match self {
            Shape::D4 => 4,
            Shape::D6 => 6,
            Shape::D8 => 8,
            Shape::D10 | Shape::D100 => 10,
            Shape::D12 => 12,
            Shape::D20 => 20,
        }
    }

    fn label(self, value: u32) -> String {
        match self {
            Shape::D10 => (value % 10).to_string(),
            Shape::D100 => format!("{:02}", (value % 10) * 10),
            _ => value.to_string(),
        }
    }

    /// Text height on a face, as a fraction of its atlas cell.
    fn text_size(self) -> f32 {
        match self {
            Shape::D4 => 0.2,
            Shape::D6 => 0.5,
            Shape::D8 => 0.3,
            Shape::D10 => 0.32,
            Shape::D12 => 0.36,
            Shape::D20 => 0.28,
            Shape::D100 => 0.26,
        }
    }
}

/// A die's geometry in its own frame.
struct Solid {
    vertices: Vec<Vec3>,
    /// Each face's corners, counterclockwise seen from outside.
    faces: Vec<Vec<usize>>,
    normals: Vec<Vec3>,
    /// Directions the result is read from, with their values: the face normals, or a d4's
    /// corners (its number is the one at the top).
    reads: Vec<(Vec3, u32)>,
    /// Rotations that map the solid onto itself.
    symmetries: Vec<Quat>,
}

const PHI: f32 = std::f32::consts::GOLDEN_RATIO;

fn signs3(x: f32, y: f32, z: f32) -> Vec<Vec3> {
    let mut out = Vec::new();
    for sx in [1.0, -1.0] {
        for sy in [1.0, -1.0] {
            for sz in [1.0, -1.0] {
                let v = Vec3::new(x * sx, y * sy, z * sz);
                if !out.contains(&v) {
                    out.push(v);
                }
            }
        }
    }
    out
}

/// Every cyclic permutation of (x, y, z) with every sign combination.
fn cyclic(x: f32, y: f32, z: f32) -> Vec<Vec3> {
    let mut out = signs3(x, y, z);
    out.extend(signs3(z, x, y));
    out.extend(signs3(y, z, x));
    out.dedup();
    out
}

/// An icosahedron, and the dodecahedron dual to it (pointing at its faces) when `dual`.
fn icosahedron(dual: bool) -> Vec<Vec3> {
    if dual {
        let mut v = signs3(1.0, 1.0, 1.0);
        v.extend(cyclic(0.0, PHI, 1.0 / PHI));
        v
    } else {
        cyclic(0.0, 1.0, PHI)
    }
}

/// A dodecahedron, and the icosahedron dual to it (pointing at its faces) when `dual`.
fn dodecahedron(dual: bool) -> Vec<Vec3> {
    if dual {
        cyclic(0.0, PHI, 1.0)
    } else {
        let mut v = signs3(1.0, 1.0, 1.0);
        v.extend(cyclic(0.0, 1.0 / PHI, PHI));
        v
    }
}

fn axes() -> Vec<Vec3> {
    cyclic(0.0, 0.0, 1.0)
}

/// A d10: two apexes on the y axis and two staggered rings of five, with ring heights that
/// keep each kite-shaped face flat.
fn trapezohedron() -> (Vec<Vec3>, Vec<Vec3>) {
    let h = 1.0;
    let c = 36f32.to_radians().cos();
    let ring = h * (1.0 - c) / (1.0 + c);
    let at = |k: usize, offset: f32, y: f32| {
        let a = (k as f32 * 72.0 + offset).to_radians();
        Vec3::new(a.cos(), y, a.sin())
    };
    let top = Vec3::new(0.0, h, 0.0);
    let bottom = Vec3::new(0.0, -h, 0.0);
    let mut vertices = vec![top, bottom];
    let mut normals = Vec::new();
    for k in 0..5 {
        vertices.push(at(k, 0.0, ring));
        vertices.push(at(k, 36.0, -ring));
        for (apex, a, b) in [
            (top, at(k, 0.0, ring), at(k + 1, 0.0, ring)),
            (bottom, at(k, 36.0, -ring), at(k + 1, 36.0, -ring)),
        ] {
            let n = (a - apex).cross(b - apex);
            // Outward, away from the center.
            normals.push(if n.dot(apex) > 0.0 { n } else { -n });
        }
    }
    (vertices, normals)
}

/// Rotations that permute `vertices`, found by mapping one vertex pair onto every other
/// pair that matches it in lengths and spacing.
fn symmetries(vertices: &[Vec3]) -> Vec<Quat> {
    let a = vertices[0];
    let b = *vertices
        .iter()
        .filter(|v| v.cross(a).length() > 1e-3)
        .min_by(|p, q| p.distance(a).total_cmp(&q.distance(a)))
        .expect("a solid has non-parallel vertices");
    let frame = |a: Vec3, b: Vec3| {
        let x = a.normalize();
        let z = a.cross(b).normalize();
        Mat3::from_cols(x, z.cross(x), z)
    };
    let from = frame(a, b).transpose();
    let close = |x: f32, y: f32| (x - y).abs() < 1e-4;
    let mut out: Vec<Quat> = Vec::new();
    for &a2 in vertices.iter().filter(|v| close(v.length(), a.length())) {
        for &b2 in vertices.iter().filter(|v| close(v.length(), b.length())) {
            if !close(a2.distance(b2), a.distance(b)) {
                continue;
            }
            let m = frame(a2, b2) * from;
            let permutes = vertices
                .iter()
                .all(|v| vertices.iter().any(|w| (m * *v).distance(*w) < 1e-3));
            let q = Quat::from_mat3(&m).normalize();
            if permutes && !out.iter().any(|p| p.dot(q).abs() > 0.9999) {
                out.push(q);
            }
        }
    }
    out
}

/// Numbers the directions so that opposite ones add up to sides + 1, like real dice.
fn number(directions: &[Vec3], sides: u32) -> Vec<u32> {
    let mut values = vec![0; directions.len()];
    let mut next = 1;
    for i in 0..directions.len() {
        if values[i] != 0 {
            continue;
        }
        values[i] = next;
        if let Some(j) = (0..directions.len())
            .find(|&j| values[j] == 0 && directions[j].dot(directions[i]) < -0.999)
        {
            values[j] = sides + 1 - next;
        }
        next += 1;
    }
    values
}

impl Solid {
    fn new(shape: Shape) -> Solid {
        let (vertices, normals, size) = match shape {
            Shape::D4 => {
                let v = vec![
                    Vec3::new(1.0, 1.0, 1.0),
                    Vec3::new(1.0, -1.0, -1.0),
                    Vec3::new(-1.0, 1.0, -1.0),
                    Vec3::new(-1.0, -1.0, 1.0),
                ];
                let n = v.iter().map(|v| -*v).collect();
                (v, n, 1.3)
            }
            Shape::D6 => (signs3(1.0, 1.0, 1.0), axes(), 0.95),
            Shape::D8 => (axes(), signs3(1.0, 1.0, 1.0), 1.1),
            Shape::D10 | Shape::D100 => {
                let (v, n) = trapezohedron();
                (v, n, 1.05)
            }
            Shape::D12 => (dodecahedron(false), dodecahedron(true), 1.05),
            Shape::D20 => (icosahedron(false), icosahedron(true), 1.1),
        };
        let radius = vertices[0].length();
        let vertices: Vec<Vec3> = vertices.iter().map(|v| *v * (size / radius)).collect();
        let normals: Vec<Vec3> = normals.iter().map(|n| n.normalize()).collect();

        // A face is every vertex furthest along its normal, sorted around it.
        let faces = normals
            .iter()
            .map(|&n| {
                let top = vertices.iter().map(|v| v.dot(n)).fold(f32::MIN, f32::max);
                let mut face: Vec<usize> = (0..vertices.len())
                    .filter(|&i| vertices[i].dot(n) > top - 1e-4)
                    .collect();
                let center = face.iter().map(|&i| vertices[i]).sum::<Vec3>() / face.len() as f32;
                let e1 = (vertices[face[0]] - center).normalize();
                let e2 = n.cross(e1);
                face.sort_by(|&p, &q| {
                    let angle = |i: usize| {
                        let d = vertices[i] - center;
                        d.dot(e2).atan2(d.dot(e1))
                    };
                    angle(p).total_cmp(&angle(q))
                });
                face
            })
            .collect();

        let read_dirs: Vec<Vec3> = if shape == Shape::D4 {
            vertices.iter().map(|v| v.normalize()).collect()
        } else {
            normals.clone()
        };
        let values = number(&read_dirs, shape.sides());
        Solid {
            symmetries: symmetries(&vertices),
            vertices,
            faces,
            normals,
            reads: read_dirs.into_iter().zip(values).collect(),
        }
    }

    /// The read direction pointing most nearly along `up`, and how nearly (1 = exactly).
    fn top(&self, rotation: Quat, up: Vec3) -> (usize, f32) {
        self.reads
            .iter()
            .enumerate()
            .map(|(i, (d, _))| (i, (rotation * *d).dot(up)))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .expect("a solid has faces")
    }

    /// A symmetry that turns the read direction `from` onto `to`.
    fn turning(&self, from: usize, to: usize) -> Quat {
        let (a, b) = (self.reads[from].0, self.reads[to].0);
        *self
            .symmetries
            .iter()
            .max_by(|p, q| (**p * a).dot(b).total_cmp(&(**q * a).dot(b)))
            .expect("a solid has symmetries")
    }

    fn read_of(&self, value: u32) -> usize {
        let value = value.clamp(1, self.reads.len() as u32);
        self.reads
            .iter()
            .position(|(_, v)| *v == value)
            .unwrap_or(0)
    }
}

/// Text drawn on a face's atlas cell, in cell coordinates (0 to 1, y down).
#[derive(Debug, Serialize)]
pub struct Label {
    text: String,
    x: f32,
    y: f32,
    /// Clockwise, in degrees.
    angle: f32,
    size: f32,
    underline: bool,
}

/// A die's mesh for a ProceduralMesh: flat-shaded faces, each mapped onto its own cell of
/// a number atlas that DieAtlas.qml draws from `cells`.
#[derive(Debug, Serialize)]
pub struct Mesh {
    positions: Vec<f32>,
    normals: Vec<f32>,
    uvs: Vec<f32>,
    indexes: Vec<u32>,
    columns: u32,
    rows: u32,
    cells: Vec<Vec<Label>>,
}

pub fn mesh(shape: Shape) -> Mesh {
    let solid = Solid::new(shape);
    let count = solid.faces.len() as u32;
    let columns = (count as f32).sqrt().ceil() as u32;
    let rows = count.div_ceil(columns);
    let mut mesh = Mesh {
        positions: Vec::new(),
        normals: Vec::new(),
        uvs: Vec::new(),
        indexes: Vec::new(),
        columns,
        rows,
        cells: Vec::new(),
    };
    let label = |text: String, x: f32, y: f32, angle: f32| {
        let underline = shape.sides() >= 8 && (text == "6" || text == "9");
        Label {
            text,
            x,
            y,
            angle,
            size: shape.text_size(),
            underline,
        }
    };
    for (f, face) in solid.faces.iter().enumerate() {
        let n = solid.normals[f];
        let center = face.iter().map(|&i| solid.vertices[i]).sum::<Vec3>() / face.len() as f32;
        // Numbers stand upright toward a d10's apex, or toward the face's first corner.
        let reference = face
            .iter()
            .copied()
            .max_by(|&p, &q| {
                solid.vertices[p]
                    .y
                    .abs()
                    .total_cmp(&solid.vertices[q].y.abs())
            })
            .filter(|_| matches!(shape, Shape::D10 | Shape::D100))
            .unwrap_or(face[0]);
        let v_axis = (solid.vertices[reference] - center).normalize();
        let u_axis = v_axis.cross(n);
        let flat = |p: Vec3| ((p - center).dot(u_axis), (p - center).dot(v_axis));
        let reach = face
            .iter()
            .map(|&i| {
                let (u, v) = flat(solid.vertices[i]);
                u.hypot(v)
            })
            .fold(0.0, f32::max);
        let scale = 0.47 / reach;
        let in_cell = |p: Vec3| {
            let (u, v) = flat(p);
            (0.5 + u * scale, 0.5 - v * scale)
        };

        let (column, row) = (f as u32 % columns, f as u32 / columns);
        let first = (mesh.positions.len() / 3) as u32;
        for &i in face {
            let p = solid.vertices[i];
            let (x, y) = in_cell(p);
            mesh.positions.extend([p.x, p.y, p.z]);
            mesh.normals.extend([n.x, n.y, n.z]);
            mesh.uvs.extend([
                (column as f32 + x) / columns as f32,
                (row as f32 + y) / rows as f32,
            ]);
        }
        for k in 1..face.len() as u32 - 1 {
            mesh.indexes.extend([first, first + k, first + k + 1]);
        }

        let cell = if shape == Shape::D4 {
            // A d4 face shows each corner's number near that corner, pointing at it.
            face.iter()
                .map(|&i| {
                    let (x, y) = in_cell(solid.vertices[i]);
                    let (dx, dy) = (x - 0.5, y - 0.5);
                    let angle = dx.atan2(-dy).to_degrees();
                    let value = solid.reads[i].1;
                    label(shape.label(value), 0.5 + dx * 0.55, 0.5 + dy * 0.55, angle)
                })
                .collect()
        } else {
            vec![label(shape.label(solid.reads[f].1), 0.5, 0.5, 0.0)]
        };
        mesh.cells.push(cell);
    }
    mesh
}

/// One die to throw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DieRoll {
    pub shape: Shape,
    pub value: u32,
    /// False for a die the roll dropped, such as the low d20 at advantage.
    pub kept: bool,
}

#[derive(Debug, Serialize)]
pub struct ThrownDie {
    pub shape: Shape,
    pub value: u32,
    pub kept: bool,
    /// The mesh's rotation inside its body, as x, y, z, w.
    pub offset: [f32; 4],
    /// The body's position and rotation each frame: x, y, z, qx, qy, qz, qw.
    pub frames: Vec<[f32; 7]>,
}

#[derive(Debug, Serialize)]
pub struct Throw {
    pub dice: Vec<ThrownDie>,
    pub fps: u32,
    /// Half the table's width (x) and depth (z). The floor is at y = 0.
    pub width: f32,
    pub depth: f32,
}

/// The most dice thrown at once; a bigger roll throws its first `MAX_DICE`.
pub const MAX_DICE: usize = 20;

/// The dice to throw for a roll, following each term's dice, kept or not. A d100 is two
/// d10s (tens and units); dice no solid fits, such as a d3, aren't thrown.
pub fn dice_of(roll: &RollResult) -> Vec<DieRoll> {
    let mut dice = Vec::new();
    for term in &roll.terms {
        let sides = term
            .label
            .split_once('d')
            .map(|(_, rest)| {
                rest.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
            })
            .and_then(|s| s.parse::<u32>().ok());
        let shape = match sides {
            Some(4) => Shape::D4,
            Some(6) => Shape::D6,
            Some(8) => Shape::D8,
            Some(10) => Shape::D10,
            Some(12) => Shape::D12,
            Some(20) => Shape::D20,
            Some(100) => Shape::D100,
            _ => continue,
        };
        for &(value, kept) in &term.rolls {
            if shape == Shape::D100 {
                let digit = |d: u32| if d == 0 { 10 } else { d };
                dice.push(DieRoll {
                    shape,
                    value: digit(value / 10 % 10),
                    kept,
                });
                dice.push(DieRoll {
                    shape: Shape::D10,
                    value: digit(value % 10),
                    kept,
                });
            } else {
                dice.push(DieRoll { shape, value, kept });
            }
        }
    }
    dice.truncate(MAX_DICE);
    dice
}

pub const FPS: u32 = 60;
const MAX_FRAMES: usize = 5 * FPS as usize;
const GRAVITY: f32 = -45.0;

fn random_rotation(rng: &mut StdRng) -> Quat {
    // Uniform over all rotations (Shoemake).
    let (u1, u2, u3): (f32, f32, f32) = (rng.random(), rng.random(), rng.random());
    let tau = std::f32::consts::TAU;
    Quat::from_xyzw(
        (1.0 - u1).sqrt() * (tau * u2).sin(),
        (1.0 - u1).sqrt() * (tau * u2).cos(),
        u1.sqrt() * (tau * u3).sin(),
        u1.sqrt() * (tau * u3).cos(),
    )
}

/// Throws `dice` across a table shaped like the view (`aspect` is width / height) so that
/// each lands showing its value.
pub fn simulate(dice: &[DieRoll], aspect: f32, seed: u64) -> Throw {
    let depth = 6.0 + dice.len() as f32 * 0.1;
    let width = depth * aspect.clamp(0.6, 3.0);
    let solids: Vec<Solid> = dice.iter().map(|d| Solid::new(d.shape)).collect();
    // A die can come to rest leaning on another; try a few throws for one that lands flat.
    let mut attempt = 0;
    let (bodies, frames) = loop {
        let (bodies, frames, flat) = run(&solids, width, depth, seed.wrapping_add(attempt));
        attempt += 1;
        if flat || attempt == 8 {
            break (bodies, frames);
        }
    };
    let dice = dice
        .iter()
        .zip(&solids)
        .zip(bodies)
        .map(|((die, solid), rotation)| {
            let landed = solid.top(rotation, Vec3::Y).0;
            let offset = solid.turning(solid.read_of(die.value), landed);
            ThrownDie {
                shape: die.shape,
                value: die.value,
                kept: die.kept,
                offset: offset.to_array(),
                frames: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    let mut dice = dice;
    for (i, die) in dice.iter_mut().enumerate() {
        die.frames = frames.iter().map(|frame| frame[i]).collect();
    }
    Throw {
        dice,
        fps: FPS,
        width,
        depth,
    }
}

/// Runs one throw. Returns each body's final rotation, every frame, and whether every die
/// came to rest flat.
fn run(
    solids: &[Solid],
    width: f32,
    depth: f32,
    seed: u64,
) -> (Vec<Quat>, Vec<Vec<[f32; 7]>>, bool) {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();

    // The floor, four walls and a lid.
    let table = bodies.insert(RigidBodyBuilder::fixed());
    let wall = |hx: f32, hy: f32, hz: f32, at: Vec3| {
        ColliderBuilder::cuboid(hx, hy, hz)
            .translation(at)
            .friction(0.6)
            .restitution(0.35)
    };
    let (w, d) = (width, depth);
    for c in [
        wall(w + 2.0, 1.0, d + 2.0, Vec3::new(0.0, -1.0, 0.0)),
        wall(w + 2.0, 1.0, d + 2.0, Vec3::new(0.0, 16.0, 0.0)),
        wall(1.0, 9.0, d + 2.0, Vec3::new(-w - 1.0, 8.0, 0.0)),
        wall(1.0, 9.0, d + 2.0, Vec3::new(w + 1.0, 8.0, 0.0)),
        wall(w + 2.0, 9.0, 1.0, Vec3::new(0.0, 8.0, -d - 1.0)),
        wall(w + 2.0, 9.0, 1.0, Vec3::new(0.0, 8.0, d + 1.0)),
    ] {
        colliders.insert_with_parent(c, table, &mut bodies);
    }

    // Thrown in from the left or right, spread out so they don't start overlapping.
    let side = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    let mut starts: Vec<Vec3> = Vec::new();
    let mut handles = Vec::new();
    for solid in solids {
        let mut at = Vec3::ZERO;
        for _ in 0..200 {
            at = Vec3::new(
                side * rng.random_range(w * 0.35..(w - 1.4).max(w * 0.35 + 0.1)),
                rng.random_range(2.0..9.0),
                rng.random_range(-(d - 1.4)..(d - 1.4)),
            );
            if starts.iter().all(|s| s.distance(at) > 2.4) {
                break;
            }
        }
        starts.push(at);
        let body = RigidBodyBuilder::dynamic()
            .pose(Pose::from_parts(at, random_rotation(&mut rng)))
            .linvel(Vec3::new(
                -side * rng.random_range(10.0..22.0),
                rng.random_range(-4.0..2.0),
                rng.random_range(-6.0..6.0),
            ))
            .angvel(Vec3::new(
                rng.random_range(-25.0..25.0),
                rng.random_range(-25.0..25.0),
                rng.random_range(-25.0..25.0),
            ))
            .ccd_enabled(true)
            .build();
        let handle = bodies.insert(body);
        let hull = ColliderBuilder::convex_hull(&solid.vertices)
            .expect("die vertices form a hull")
            .density(1.0)
            .friction(0.5)
            .restitution(0.3);
        colliders.insert_with_parent(hull, handle, &mut bodies);
        handles.push(handle);
    }

    let parameters = IntegrationParameters {
        dt: 1.0 / FPS as f32,
        ..Default::default()
    };
    let mut pipeline = PhysicsPipeline::new();
    let mut islands = IslandManager::new();
    let mut broad_phase = BroadPhaseBvh::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joints = ImpulseJointSet::new();
    let mut multibody_joints = MultibodyJointSet::new();
    let mut soft_bodies = SoftBodySet::new();
    let mut ccd = CCDSolver::new();
    let mut frames = Vec::new();
    while frames.len() < MAX_FRAMES {
        pipeline.step(
            Vec3::new(0.0, GRAVITY, 0.0),
            &parameters,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut bodies,
            &mut colliders,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut soft_bodies,
            &mut ccd,
            &(),
            &(),
        );
        let round = |x: f32| (x * 1e4).round() / 1e4;
        frames.push(
            handles
                .iter()
                .map(|&h| {
                    let b = &bodies[h];
                    let (t, q) = (b.translation(), *b.rotation());
                    [t.x, t.y, t.z, q.x, q.y, q.z, q.w].map(round)
                })
                .collect::<Vec<_>>(),
        );
        if handles.iter().all(|&h| bodies[h].is_sleeping()) {
            break;
        }
    }
    let rotations: Vec<Quat> = handles.iter().map(|&h| *bodies[h].rotation()).collect();
    let flat = solids
        .iter()
        .zip(&rotations)
        .all(|(solid, &q)| solid.top(q, Vec3::Y).1 > 0.98);
    (rotations, frames, flat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solids_have_the_right_faces_and_symmetries() {
        for (shape, faces, symmetries) in [
            (Shape::D4, 4, 12),
            (Shape::D6, 6, 24),
            (Shape::D8, 8, 24),
            (Shape::D10, 10, 10),
            (Shape::D12, 12, 60),
            (Shape::D20, 20, 60),
        ] {
            let s = Solid::new(shape);
            assert_eq!(s.faces.len(), faces, "{shape:?} faces");
            assert_eq!(s.symmetries.len(), symmetries, "{shape:?} symmetries");
            let mut values: Vec<u32> = s.reads.iter().map(|r| r.1).collect();
            values.sort();
            assert_eq!(
                values,
                (1..=shape.sides()).collect::<Vec<_>>(),
                "{shape:?} values"
            );
            // Any number can be turned onto any face.
            for from in 0..s.reads.len() {
                for to in 0..s.reads.len() {
                    let q = s.turning(from, to);
                    assert!(
                        (q * s.reads[from].0).dot(s.reads[to].0) > 0.999,
                        "{shape:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn faces_wind_outward() {
        for shape in Shape::ALL {
            let s = Solid::new(shape);
            for (face, n) in s.faces.iter().zip(&s.normals) {
                let [a, b, c] = [0, 1, 2].map(|k| s.vertices[face[k]]);
                assert!((b - a).cross(c - a).dot(*n) > 0.0, "{shape:?}");
            }
        }
    }

    #[test]
    fn every_value_lands_face_up() {
        for shape in Shape::ALL {
            let solid = Solid::new(shape);
            for value in 1..=shape.sides() {
                let die = DieRoll {
                    shape,
                    value,
                    kept: true,
                };
                let thrown = simulate(&[die], 1.6, value as u64);
                let d = &thrown.dice[0];
                let last = d.frames.last().unwrap();
                let body = Quat::from_xyzw(last[3], last[4], last[5], last[6]);
                let mesh = Quat::from_array(d.offset);
                let (top, lean) = solid.top(body * mesh, Vec3::Y);
                assert_eq!(solid.reads[top].1, value, "{shape:?} showing {value}");
                assert!(lean > 0.95, "{shape:?} showing {value} is lying flat");
                assert!(last[1] < 2.0, "{shape:?} came to rest on the table");
            }
        }
    }

    #[test]
    fn many_dice_land_on_their_values() {
        let dice: Vec<DieRoll> = (0..20)
            .map(|i| DieRoll {
                shape: Shape::ALL[i % 7],
                value: (i as u32 % 4) + 1,
                kept: true,
            })
            .collect();
        let thrown = simulate(&dice, 1.0, 7);
        for (die, d) in dice.iter().zip(&thrown.dice) {
            let solid = Solid::new(die.shape);
            let last = d.frames.last().unwrap();
            let rotation =
                Quat::from_xyzw(last[3], last[4], last[5], last[6]) * Quat::from_array(d.offset);
            assert_eq!(solid.reads[solid.top(rotation, Vec3::Y).0].1, die.value);
        }
    }

    #[test]
    fn throws_are_repeatable() {
        let dice = [DieRoll {
            shape: Shape::D20,
            value: 17,
            kept: true,
        }; 3];
        let a = simulate(&dice, 1.5, 42);
        let b = simulate(&dice, 1.5, 42);
        assert_eq!(a.dice[1].frames, b.dice[1].frames);
        assert_eq!(a.dice[1].offset, b.dice[1].offset);
    }

    #[test]
    fn rolls_become_dice() {
        use dnd_core::dice::{RollMode, TermResult};
        let term = |label: &str, rolls: Vec<(u32, bool)>| TermResult {
            sign: 1,
            label: label.into(),
            rolls,
            value: 0,
            source: String::new(),
            damage_type: String::new(),
        };
        let roll = RollResult {
            expression: String::new(),
            mode: RollMode::Advantage,
            terms: vec![
                term("2d20kh1", vec![(3, false), (18, true)]),
                term("1d100", vec![(100, true)]),
                term("1d100", vec![(47, true)]),
                term("1d3", vec![(2, true)]),
                term("5", vec![]),
            ],
            total: 0,
            natural: None,
        };
        let die = |shape, value, kept| DieRoll { shape, value, kept };
        assert_eq!(
            dice_of(&roll),
            vec![
                die(Shape::D20, 3, false),
                die(Shape::D20, 18, true),
                die(Shape::D100, 10, true),
                die(Shape::D10, 10, true),
                die(Shape::D100, 4, true),
                die(Shape::D10, 7, true),
            ]
        );
    }

    #[test]
    fn meshes_have_a_cell_per_face() {
        for shape in Shape::ALL {
            let m = mesh(shape);
            assert_eq!(m.cells.len(), Solid::new(shape).faces.len());
            assert!(m.uvs.iter().all(|u| (0.0..=1.0).contains(u)), "{shape:?}");
            assert_eq!(m.positions.len() / 3, m.uvs.len() / 2);
        }
        let d4 = mesh(Shape::D4);
        assert!(d4.cells.iter().all(|c| c.len() == 3));
    }
}
