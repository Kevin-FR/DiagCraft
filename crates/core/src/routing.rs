//! Short automatic orthogonal routes that respect explicit attachment sides.
//! Only the endpoint blocks are considered; intermediate obstacles remain editable.
use crate::document::{Node, Point, Port};
type Bounds = [f32; 4];
fn bounds(n: &Node) -> Bounds {
    [
        n.position[0],
        n.position[1],
        n.position[0] + n.size[0],
        n.position[1] + n.size[1],
    ]
}
fn port(port: Port, center: Point, towards: Point) -> Port {
    if port != Port::Auto {
        return port;
    }
    let delta = [towards[0] - center[0], towards[1] - center[1]];
    if delta[0].abs() >= delta[1].abs() {
        if delta[0] >= 0. {
            Port::Right
        } else {
            Port::Left
        }
    } else if delta[1] >= 0. {
        Port::Bottom
    } else {
        Port::Top
    }
}
fn crosses(a: Point, b: Point, r: Bounds) -> bool {
    // Open interiors: a line on the border does not traverse the block.
    if (a[0] - b[0]).abs() < 0.001 {
        a[0] > r[0] + 0.01
            && a[0] < r[2] - 0.01
            && a[1].min(b[1]) < r[3] - 0.01
            && a[1].max(b[1]) > r[1] + 0.01
    } else if (a[1] - b[1]).abs() < 0.001 {
        a[1] > r[1] + 0.01
            && a[1] < r[3] - 0.01
            && a[0].min(b[0]) < r[2] - 0.01
            && a[0].max(b[0]) > r[0] + 0.01
    } else {
        true
    }
}
fn stub(at: Point, node: Option<&Node>, p: Port, other: Option<&Node>) -> Point {
    let Some(node) = node else {
        return at;
    };
    let r = bounds(node);
    let (axis, sign, edge) = match p {
        Port::Left => (0, -1., r[0]),
        Port::Right => (0, 1., r[2]),
        Port::Top => (1, -1., r[1]),
        _ => (1, 1., r[3]),
    };
    let mut distance = (edge - at[axis]).abs() + 24.;
    if let Some(other) = other {
        let b = bounds(other);
        let other_axis = 1 - axis;
        if at[other_axis] > b[other_axis] && at[other_axis] < b[other_axis + 2] {
            let gap = if sign > 0. {
                b[axis] - at[axis]
            } else {
                at[axis] - b[axis + 2]
            };
            if gap > 0. {
                distance = distance.min(gap / 3.);
            }
        }
    }
    let mut out = at;
    out[axis] += sign * distance;
    out
}
pub(crate) fn orthogonal(
    a: Option<&Node>,
    b: Option<&Node>,
    ac: Point,
    bc: Point,
    from: Port,
    to: Port,
) -> Vec<Point> {
    let from = port(from, ac, bc);
    let to = port(to, bc, ac);
    let start = a.map_or(ac, |n| n.port_position(from, bc));
    let end = b.map_or(bc, |n| n.port_position(to, ac));
    let s = stub(start, a, from, b);
    let t = stub(end, b, to, a);
    let obstacles = a.into_iter().chain(b).map(bounds).collect::<Vec<_>>();
    let minx = obstacles
        .iter()
        .map(|r| r[0])
        .fold(s[0].min(t[0]), f32::min)
        - 24.;
    let maxx = obstacles
        .iter()
        .map(|r| r[2])
        .fold(s[0].max(t[0]), f32::max)
        + 24.;
    let miny = obstacles
        .iter()
        .map(|r| r[1])
        .fold(s[1].min(t[1]), f32::min)
        - 24.;
    let maxy = obstacles
        .iter()
        .map(|r| r[3])
        .fold(s[1].max(t[1]), f32::max)
        + 24.;
    let mut candidates = vec![vec![s, [t[0], s[1]], t], vec![s, [s[0], t[1]], t]];
    for x in [minx, maxx, (s[0] + t[0]) / 2.] {
        candidates.push(vec![s, [x, s[1]], [x, t[1]], t]);
    }
    for y in [miny, maxy, (s[1] + t[1]) / 2.] {
        candidates.push(vec![s, [s[0], y], [t[0], y], t]);
    }
    for x in [minx, maxx] {
        for y in [miny, maxy] {
            candidates.push(vec![s, [x, s[1]], [x, y], [t[0], y], t]);
            candidates.push(vec![s, [s[0], y], [x, y], [x, t[1]], t]);
        }
    }
    let middle = candidates
        .into_iter()
        .filter(|path| {
            path.windows(2)
                .all(|p| obstacles.iter().all(|r| !crosses(p[0], p[1], *r)))
        })
        .min_by(|a, b| score(a).total_cmp(&score(b)))
        .unwrap_or_else(|| vec![s, [t[0], s[1]], t]);
    let mut route = vec![start];
    route.extend(middle);
    route.push(end);
    route
}
fn score(path: &[Point]) -> f32 {
    path.windows(2)
        .map(|p| (p[0][0] - p[1][0]).abs() + (p[0][1] - p[1][1]).abs())
        .sum::<f32>()
        + path.len() as f32 * 0.1
}
