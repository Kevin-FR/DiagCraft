//! Deterministic orthogonal routing shared by the canvas, exports and agents.
//! Manual anchors are immutable; only the paths between them are computed.
use crate::document::{Connector, ConnectorKind, Document, Node, Point, Port, ShapeKind};
use serde::Serialize;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

type Bounds = [f32; 4];
const EPS: f32 = 0.01;
const LANE: f32 = 8.;
const BEND: f32 = 24.;
const GRID_OBSTACLES: usize = 34;
const MAX_SEARCH: usize = 24_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStatus {
    #[default]
    Clear,
    /// Straight links intentionally bypass orthogonal obstacle routing.
    Direct,
    /// An anchor/port lies inside another block, or no collision-free path exists.
    Constrained,
    /// The bounded search could not represent/find a path through a dense layout.
    Limited,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct RoutedConnector {
    pub points: Vec<Point>,
    pub status: RouteStatus,
}
impl RoutedConnector {
    pub fn label_position(&self, t: f32) -> Point {
        label_position(&self.points, t)
    }
}
pub fn label_position(route: &[Point], t: f32) -> Point {
    let total: f32 = route.windows(2).map(|p| distance(p[0], p[1])).sum();
    let mut remain = total * t;
    for p in route.windows(2) {
        let length = distance(p[0], p[1]);
        if remain <= length && length > EPS {
            let t = remain / length;
            return [
                p[0][0]
                    + (p[1][0] - p[0][0]) * t
                    + if (p[0][0] - p[1][0]).abs() < EPS {
                        12.
                    } else {
                        0.
                    },
                p[0][1] + (p[1][1] - p[0][1]) * t - 10.,
            ];
        }
        remain -= length;
    }
    route.last().copied().unwrap_or([0., 0.])
}
fn bounds(n: &Node) -> Bounds {
    rect(n.position, n.size)
}
fn rect(p: Point, size: Point) -> Bounds {
    [p[0], p[1], p[0] + size[0], p[1] + size[1]]
}
fn contains(r: Bounds, p: Point) -> bool {
    p[0] > r[0] + EPS && p[0] < r[2] - EPS && p[1] > r[1] + EPS && p[1] < r[3] - EPS
}
fn crosses(a: Point, b: Point, r: Bounds) -> bool {
    if (a[0] - b[0]).abs() < EPS {
        a[0] > r[0] + EPS
            && a[0] < r[2] - EPS
            && a[1].min(b[1]) < r[3] - EPS
            && a[1].max(b[1]) > r[1] + EPS
    } else {
        a[1] > r[1] + EPS
            && a[1] < r[3] - EPS
            && a[0].min(b[0]) < r[2] - EPS
            && a[0].max(b[0]) > r[0] + EPS
    }
}
fn distance(a: Point, b: Point) -> f32 {
    (a[0] - b[0]).abs() + (a[1] - b[1]).abs()
}
fn port(p: Port, center: Point, towards: Point) -> Port {
    if p != Port::Auto {
        return p;
    }
    let d = [towards[0] - center[0], towards[1] - center[1]];
    if d[0].abs() >= d[1].abs() {
        if d[0] >= 0. { Port::Right } else { Port::Left }
    } else if d[1] >= 0. {
        Port::Bottom
    } else {
        Port::Top
    }
}
fn axis(p: Port) -> usize {
    if matches!(p, Port::Left | Port::Right) {
        0
    } else {
        1
    }
}
fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut out: Vec<Point> = vec![];
    for p in points {
        if out.last().is_some_and(|q| *q == p) {
            continue;
        }
        while out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            if ((a[0] == b[0] && b[0] == p[0]) || (a[1] == b[1] && b[1] == p[1]))
                && (b[0] - a[0]) * (p[0] - b[0]) + (b[1] - a[1]) * (p[1] - b[1]) >= 0.
            {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

/// Build once per page/batch. Hidden objects and free text do not obstruct links.
/// Rectangular envelopes deliberately give curved/concave blocks breathing room.
pub struct RoutingContext<'a> {
    nodes: HashMap<u64, &'a Node>,
    obstacles: Vec<(u64, Bounds)>,
}
impl<'a> RoutingContext<'a> {
    pub fn new(doc: &'a Document) -> Self {
        let view = doc.organization_view();
        Self {
            nodes: doc.nodes.iter().map(|n| (n.id, n)).collect(),
            obstacles: doc
                .nodes
                .iter()
                .filter(|n| {
                    !matches!(
                        n.kind,
                        ShapeKind::Text | ShapeKind::Frame | ShapeKind::Swimlane
                    ) && view.is_object_visible(n.id)
                })
                .map(|n| (n.id, bounds(n)))
                .chain(
                    doc.charts
                        .iter()
                        .filter(|c| view.is_object_visible(c.id))
                        .map(|c| (c.id, rect(c.position, c.size))),
                )
                .collect(),
        }
    }
    pub fn route(&self, e: &Connector) -> RoutedConnector {
        let a = self.nodes.get(&e.from).copied();
        let b = self.nodes.get(&e.to).copied();
        let (Some(ac), Some(bc)) = (
            a.map(Node::center).or(e.free_start),
            b.map(Node::center).or(e.free_end),
        ) else {
            return RoutedConnector::default();
        };
        if e.kind == ConnectorKind::Straight {
            return RoutedConnector {
                points: vec![
                    a.map_or(ac, |n| n.port_position(e.from_port, bc)),
                    b.map_or(bc, |n| n.port_position(e.to_port, ac)),
                ],
                status: RouteStatus::Direct,
            };
        }
        if distance(ac, bc) < EPS
            && e.from_port == Port::Auto
            && e.to_port == Port::Auto
            && e.waypoints.is_empty()
        {
            return RoutedConnector {
                points: vec![ac, bc],
                status: RouteStatus::Constrained,
            };
        }
        let from = port(e.from_port, ac, e.waypoints.first().copied().unwrap_or(bc));
        let to = port(e.to_port, bc, e.waypoints.last().copied().unwrap_or(ac));
        let start = a.map_or(ac, |n| n.port_position(from, bc));
        let end = b.map_or(bc, |n| n.port_position(to, ac));
        let s = self.stub(start, a, from);
        let t = self.stub(end, b, to);
        let mut anchors = vec![s];
        anchors.extend_from_slice(&e.waypoints);
        anchors.push(t);
        let mut status = RouteStatus::Clear;
        // Endpoint shapes need no margin: their cardinal stubs supply it. Other
        // boxes use snapped lanes; close mandatory anchors reduce only the margin.
        let clearance = LANE + e.width / 2.;
        let obstacles: Vec<_> = self
            .obstacles
            .iter()
            .map(|&(id, r)| {
                if id == e.from || id == e.to {
                    return r;
                }
                let margin = anchors
                    .iter()
                    .map(|p| {
                        (r[0] - p[0])
                            .max(p[0] - r[2])
                            .max(0.)
                            .max((r[1] - p[1]).max(p[1] - r[3]).max(0.))
                            / 2.
                    })
                    .fold(clearance, f32::min);
                if margin < clearance {
                    [r[0] - margin, r[1] - margin, r[2] + margin, r[3] + margin]
                } else {
                    [
                        (r[0] - clearance).div_euclid(LANE) * LANE,
                        (r[1] - clearance).div_euclid(LANE) * LANE,
                        ((r[2] + clearance) / LANE).ceil() * LANE,
                        ((r[3] + clearance) / LANE).ceil() * LANE,
                    ]
                }
            })
            .collect();
        let mut route = vec![start];
        let mut budget = MAX_SEARCH;
        for (i, pair) in anchors.windows(2).enumerate() {
            // A waypoint inside a block is a user constraint. Preserve it exactly,
            // allow only this leg to enter/leave that box, and report the conflict.
            let mut conflict = false;
            let allowed: Vec<_> = obstacles
                .iter()
                .copied()
                .filter(|r| {
                    let inside = contains(*r, pair[0]) || contains(*r, pair[1]);
                    conflict |= inside;
                    !inside
                })
                .collect();
            if conflict {
                status = RouteStatus::Constrained;
            }
            let start_axis = if i == 0 && a.is_some() {
                Some(axis(from))
            } else {
                None
            };
            let end_axis = if i + 2 == anchors.len() && b.is_some() {
                Some(axis(to))
            } else {
                None
            };
            let (path, limited) = leg(
                pair[0],
                pair[1],
                &allowed,
                start_axis,
                end_axis,
                &mut budget,
            );
            if limited && status == RouteStatus::Clear {
                status = RouteStatus::Limited;
            }
            // Simplify within a leg only: even collinear user anchors stay present.
            let mut part = vec![if i == 0 { start } else { pair[0] }];
            part.extend(path);
            if i + 2 == anchors.len() {
                part.push(end);
            }
            let mut part = simplify(part);
            let required = if i + 2 == anchors.len() { end } else { pair[1] };
            if part.last() != Some(&required) {
                part.push(required);
            }
            for p in part {
                if route.last().is_none_or(|q| *q != p) {
                    route.push(p);
                }
            }
        }
        // Stubs may be physically trapped by overlapping blocks. Never advertise
        // a crossing route as clear, even if the grid found a clear middle part.
        if self.obstacles.iter().any(|&(id, r)| {
            (id != e.from && crosses(start, s, r)) || (id != e.to && crosses(t, end, r))
        }) {
            status = RouteStatus::Constrained;
        }
        RoutedConnector {
            points: route,
            status,
        }
    }
    fn stub(&self, at: Point, node: Option<&Node>, p: Port) -> Point {
        let Some(n) = node else {
            return at;
        };
        let r = bounds(n);
        let ax = axis(p);
        let sign = if matches!(p, Port::Left | Port::Top) {
            -1.
        } else {
            1.
        };
        let edge = if sign < 0. { r[ax] } else { r[ax + 2] };
        let mut length = (edge - at[ax]).abs() + 24.;
        for &(id, b) in &self.obstacles {
            if id == n.id {
                continue;
            }
            if at[1 - ax] > b[1 - ax] - EPS && at[1 - ax] < b[3 - ax] + EPS {
                let gap = if sign > 0. {
                    b[ax] - at[ax]
                } else {
                    at[ax] - b[ax + 2]
                };
                if gap > EPS {
                    length = length.min(gap / 3.);
                }
            }
        }
        let mut out = at;
        out[ax] += sign * length;
        out
    }
}
fn cost(path: &[Point], first: Option<usize>, last: Option<usize>) -> f32 {
    let mut score = 0.;
    let mut prev = first;
    for p in path.windows(2) {
        if distance(p[0], p[1]) < EPS {
            continue;
        }
        let ax = if (p[0][0] - p[1][0]).abs() < EPS {
            1
        } else {
            0
        };
        score += distance(p[0], p[1])
            + if prev.is_some_and(|v| v != ax) {
                BEND
            } else {
                0.
            };
        prev = Some(ax);
    }
    score
        + if last.is_some() && prev.is_some() && last != prev {
            BEND
        } else {
            0.
        }
}
fn clear(path: &[Point], obstacles: &[Bounds]) -> bool {
    path.windows(2)
        .all(|p| obstacles.iter().all(|r| !crosses(p[0], p[1], *r)))
}
fn leg(
    s: Point,
    t: Point,
    obstacles: &[Bounds],
    first: Option<usize>,
    last: Option<usize>,
    budget: &mut usize,
) -> (Vec<Point>, bool) {
    if s == t {
        return (vec![s], false);
    }
    let mut outer = [
        s[0].min(t[0]),
        s[1].min(t[1]),
        s[0].max(t[0]),
        s[1].max(t[1]),
    ];
    for r in obstacles {
        outer[0] = outer[0].min(r[0]);
        outer[1] = outer[1].min(r[1]);
        outer[2] = outer[2].max(r[2]);
        outer[3] = outer[3].max(r[3]);
    }
    for v in &mut outer[..2] {
        *v -= LANE;
    }
    for v in &mut outer[2..] {
        *v += LANE;
    }
    let mut candidates = vec![vec![s, [t[0], s[1]], t], vec![s, [s[0], t[1]], t]];
    let x = (s[0] + t[0]) / 2.;
    let y = (s[1] + t[1]) / 2.;
    candidates.push(vec![s, [x, s[1]], [x, t[1]], t]);
    candidates.push(vec![s, [s[0], y], [t[0], y], t]);
    let mut nearby = obstacles.to_vec();
    let proximity = |r: &Bounds| {
        (s[0].min(t[0]) - r[2]).max(r[0] - s[0].max(t[0])).max(0.)
            + (s[1].min(t[1]) - r[3]).max(r[1] - s[1].max(t[1])).max(0.)
    };
    nearby.sort_by(|a, b| {
        proximity(a).total_cmp(&proximity(b)).then_with(|| {
            a.iter()
                .zip(b)
                .map(|(a, b)| a.total_cmp(b))
                .find(|c| *c != Ordering::Equal)
                .unwrap_or(Ordering::Equal)
        })
    });
    for r in nearby.iter().take(8) {
        for y in [r[1], r[3]] {
            candidates.push(vec![s, [s[0], y], [t[0], y], t]);
        }
        for x in [r[0], r[2]] {
            candidates.push(vec![s, [x, s[1]], [x, t[1]], t]);
        }
    }
    for x in [outer[0], outer[2]] {
        for y in [outer[1], outer[3]] {
            candidates.push(vec![s, [x, s[1]], [x, y], [t[0], y], t]);
            candidates.push(vec![s, [s[0], y], [x, y], [x, t[1]], t]);
        }
    }
    let candidates: Vec<_> = candidates
        .into_iter()
        .filter(|p| clear(p, obstacles))
        .map(simplify)
        .collect();
    let shortest = candidates
        .iter()
        .map(|p| cost(p, first, last))
        .fold(f32::INFINITY, f32::min);
    // A small length tolerance favours the same candidate family over flipping
    // above/below an obstacle after a one-pixel drag. No view-specific history:
    // reopening, undo, exports and CLI all compute the same route.
    let best = candidates
        .into_iter()
        .find(|p| cost(p, first, last) <= shortest + LANE * 2.);
    // A short unobstructed dogleg is already optimal; avoid building a grid.
    if let Some(ref best) = best
        && cost(best, first, last) <= distance(s, t) + BEND * 2. + EPS
    {
        return (best.clone(), false);
    }
    if *budget > 0
        && let Some(path) = search(s, t, obstacles, outer, first, last, budget)
        && best
            .as_ref()
            .is_none_or(|p| cost(&path, first, last) < cost(p, first, last) - LANE * 2. - EPS)
    {
        return (path, false);
    }
    if let Some(best) = best {
        return (best, false);
    }
    (vec![s, [t[0], s[1]], t], true)
}

// Coordinate-compressed rectilinear grid. All obstacles block graph edges, even
// when their lanes are outside the coordinate budget. Difference arrays build
// those edges in O(obstacles * log(axis) + grid), without obstacle scans per A* node.
fn search(
    s: Point,
    t: Point,
    obstacles: &[Bounds],
    outer: Bounds,
    first: Option<usize>,
    last: Option<usize>,
    budget: &mut usize,
) -> Option<Vec<Point>> {
    let corridor = [
        s[0].min(t[0]),
        s[1].min(t[1]),
        s[0].max(t[0]),
        s[1].max(t[1]),
    ];
    let mut nearby = obstacles.to_vec();
    let proximity = |r: &Bounds| {
        (corridor[0] - r[2]).max(r[0] - corridor[2]).max(0.)
            + (corridor[1] - r[3]).max(r[1] - corridor[3]).max(0.)
    };
    nearby.sort_by(|a, b| {
        proximity(a).total_cmp(&proximity(b)).then_with(|| {
            a.iter()
                .zip(b)
                .map(|(a, b)| a.total_cmp(b))
                .find(|c| *c != Ordering::Equal)
                .unwrap_or(Ordering::Equal)
        })
    });
    let mut xs = vec![s[0], t[0], outer[0], outer[2]];
    let mut ys = vec![s[1], t[1], outer[1], outer[3]];
    for r in nearby.iter().take(GRID_OBSTACLES) {
        xs.extend([r[0], r[2]]);
        ys.extend([r[1], r[3]]);
    }
    for values in [&mut xs, &mut ys] {
        values.sort_by(f32::total_cmp);
        values.dedup_by(|a, b| *a == *b);
    }
    let w = xs.len();
    let h = ys.len();
    let mut horizontal = vec![0i32; (w + 1) * (h + 1)];
    let mut vertical = horizontal.clone();
    for r in obstacles {
        let x0 = xs.partition_point(|x| *x <= r[0] + EPS);
        let x1 = xs.partition_point(|x| *x < r[2] - EPS);
        let y0 = ys.partition_point(|y| *y <= r[1] + EPS);
        let y1 = ys.partition_point(|y| *y < r[3] - EPS);
        let hx0 = xs.partition_point(|x| *x <= r[0] + EPS).saturating_sub(1);
        let hx1 = x1.min(w - 1);
        let vy0 = ys.partition_point(|y| *y <= r[1] + EPS).saturating_sub(1);
        let vy1 = y1.min(h - 1);
        mark(&mut horizontal, w + 1, [hx0, y0, hx1, y1]);
        mark(&mut vertical, w + 1, [x0, vy0, x1, vy1]);
    }
    for grid in [&mut horizontal, &mut vertical] {
        for y in 0..h {
            for x in 0..w {
                let i = y * (w + 1) + x;
                if x > 0 {
                    grid[i] += grid[i - 1];
                }
                if y > 0 {
                    grid[i] += grid[i - w - 1];
                }
                if x > 0 && y > 0 {
                    grid[i] -= grid[i - w - 2];
                }
            }
        }
    }
    let locate =
        |p: Point| ys.partition_point(|v| *v < p[1]) * w + xs.partition_point(|v| *v < p[0]);
    let start = locate(s);
    let target = locate(t);
    let mut dist = vec![f32::INFINITY; w * h * 2];
    let mut prev = vec![usize::MAX; dist.len()];
    let mut heap = BinaryHeap::new();
    for ax in 0..2 {
        let i = start * 2 + ax;
        let g = if first.is_some_and(|v| v != ax) {
            BEND
        } else {
            0.
        };
        dist[i] = g;
        heap.push(Entry {
            f: g + distance(s, t),
            g,
            state: i,
        });
    }
    while let Some(Entry { g, state, .. }) = heap.pop() {
        if g > dist[state] {
            continue;
        }
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
        let at = state / 2;
        let ax = state % 2;
        let x = at % w;
        let y = at / w;
        if at == target {
            // Target direction is included in its incoming-edge cost below.
            let mut out = vec![];
            let mut i = state;
            loop {
                let at = i / 2;
                out.push([xs[at % w], ys[at / w]]);
                if prev[i] == usize::MAX {
                    break;
                }
                i = prev[i];
            }
            out.reverse();
            return Some(simplify(out));
        }
        let moves = [
            (
                x.checked_sub(1).map(|x| y * w + x),
                0,
                x.saturating_sub(1),
                y,
            ),
            ((x + 1 < w).then_some(y * w + x + 1), 0, x, y),
            (
                y.checked_sub(1).map(|y| y * w + x),
                1,
                x,
                y.saturating_sub(1),
            ),
            ((y + 1 < h).then_some((y + 1) * w + x), 1, x, y),
        ];
        for (next, dir, bx, by) in moves {
            let Some(next) = next else {
                continue;
            };
            let blocked = if dir == 0 { &horizontal } else { &vertical };
            if blocked[by * (w + 1) + bx] > 0 {
                continue;
            }
            let p = [xs[next % w], ys[next / w]];
            let q = [xs[x], ys[y]];
            let ng = g
                + distance(p, q)
                + if ax != dir { BEND } else { 0. }
                + if next == target && last.is_some_and(|v| v != dir) {
                    BEND
                } else {
                    0.
                };
            let ns = next * 2 + dir;
            if ng + EPS < dist[ns] {
                dist[ns] = ng;
                prev[ns] = state;
                heap.push(Entry {
                    f: ng + distance(p, t),
                    g: ng,
                    state: ns,
                });
            }
        }
    }
    None
}
fn mark(grid: &mut [i32], stride: usize, [x0, y0, x1, y1]: [usize; 4]) {
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    grid[y0 * stride + x0] += 1;
    grid[y0 * stride + x1] -= 1;
    grid[y1 * stride + x0] -= 1;
    grid[y1 * stride + x1] += 1;
}
#[derive(Clone, Copy)]
struct Entry {
    f: f32,
    g: f32,
    state: usize,
}
impl PartialEq for Entry {
    fn eq(&self, o: &Self) -> bool {
        self.f == o.f && self.g == o.g && self.state == o.state
    }
}
impl Eq for Entry {}
impl Ord for Entry {
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f)
            .then_with(|| o.g.total_cmp(&self.g))
            .then_with(|| o.state.cmp(&self.state))
    }
}
impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
