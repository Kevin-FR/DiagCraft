//! Deterministic, bounded flow placement. Produces a new document for review.
use crate::document::{Document, Point, ShapeKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}
/// Page coordinates in pixels; independent from interface zoom and direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing {
    pub levels: f32,
    pub blocks: f32,
}
impl Default for Spacing {
    fn default() -> Self {
        Self {
            levels: 60.,
            blocks: 60.,
        }
    }
}

pub struct Preview {
    pub document: Document,
    pub moved: usize,
    pub fixed: usize,
    /// Estimated crossings between consecutive levels, including long-link segments.
    // Baseline is the automatic ID order, not the source drawing's geometry.
    pub crossings_before: u64,
    pub crossings_after: u64,
}

// A group is placed using one envelope while retaining all internal offsets.
// Group IDs and object IDs have separate namespaces; the sorting key is a member ID.
struct Unit {
    nodes: Vec<u64>,
    connectors: Vec<u64>,
    charts: Vec<u64>,
    drawings: Vec<u64>,
    position: Point,
    size: Point,
    geometry: Vec<(Point, Point)>,
}
impl Unit {
    // Measure the actual f32 result rather than translating the old envelope.
    // Otherwise rounding a member can change the next level/cursor on repetition.
    fn placed_extent(&self, axis: usize, origin: f32) -> f32 {
        if self.geometry.len() == 1 || origin == self.position[axis] {
            return self.size[axis];
        }
        self.geometry
            .iter()
            .map(|(point, size)| {
                translated_coordinate(point[axis], self.position[axis], origin) + size[axis]
            })
            .fold(origin, f32::max)
            - origin
    }
    fn placed_size(&self, position: Point) -> Point {
        [
            self.placed_extent(0, position[0]),
            self.placed_extent(1, position[1]),
        ]
    }
}
fn translated_coordinate(value: f32, from: f32, to: f32) -> f32 {
    if from == to {
        value
    } else {
        to + (value - from)
    }
}

fn placement_units(
    doc: &Document,
    view: &crate::organization::OrganizationView,
    selection: Option<&BTreeSet<u64>>,
) -> Vec<Unit> {
    let nodes: BTreeMap<_, _> = doc.nodes.iter().map(|n| (n.id, n)).collect();
    let connectors: BTreeMap<_, _> = doc.connectors.iter().map(|e| (e.id, e)).collect();
    let charts: BTreeMap<_, _> = doc.charts.iter().map(|c| (c.id, c)).collect();
    let drawings: BTreeMap<_, _> = doc.drawings.iter().map(|d| (d.id, d)).collect();
    let mut units: BTreeMap<u64, Unit> = nodes
        .values()
        .filter(|n| {
            n.kind != ShapeKind::Text
                && view.can_edit_object(n.id)
                && view.group_for_object(n.id).is_none()
                && selection.is_none_or(|ids| ids.contains(&n.id))
        })
        .map(|n| {
            (
                n.id,
                Unit {
                    nodes: vec![n.id],
                    connectors: vec![],
                    charts: vec![],
                    drawings: vec![],
                    position: n.position,
                    size: n.size,
                    geometry: vec![(n.position, n.size)],
                },
            )
        })
        .collect();
    for group in &doc.organization.groups {
        if group.members.iter().any(|id| !view.can_edit_object(*id))
            || selection.is_some_and(|ids| !group.members.iter().any(|id| ids.contains(id)))
        {
            continue;
        }
        let member_nodes: BTreeSet<_> = group
            .members
            .iter()
            .copied()
            .filter(|id| nodes.contains_key(id))
            .collect();
        if !member_nodes
            .iter()
            .any(|id| nodes[id].kind != ShapeKind::Text)
        {
            continue;
        }
        // A group is placeable only when every member is a node (block, image,
        // text), a chart, a drawing, or an internal connector whose two
        // endpoints are inside the group and not free. External or free
        // connectors keep the whole group fixed, because moving them would
        // break the outside connection.
        let internal_connector = |id| {
            connectors.get(id).is_some_and(|edge| {
                member_nodes.contains(&edge.from)
                    && member_nodes.contains(&edge.to)
                    && edge.free_start.is_none()
                    && edge.free_end.is_none()
            })
        };
        if group.members.iter().any(|id| {
            !member_nodes.contains(id)
                && !charts.contains_key(id)
                && !drawings.contains_key(id)
                && !internal_connector(id)
        }) {
            continue;
        }
        let member_connectors: Vec<_> = group
            .members
            .iter()
            .copied()
            .filter(|id| connectors.contains_key(id))
            .collect();
        let member_charts: Vec<_> = group
            .members
            .iter()
            .copied()
            .filter(|id| charts.contains_key(id))
            .collect();
        let member_drawings: Vec<_> = group
            .members
            .iter()
            .copied()
            .filter(|id| drawings.contains_key(id))
            .collect();
        let mut geometry = Vec::new();
        let mut min = [f32::INFINITY; 2];
        let mut max = [f32::NEG_INFINITY; 2];
        for id in &member_nodes {
            let node = nodes[id];
            geometry.push((node.position, node.size));
            for axis in 0..2 {
                min[axis] = min[axis].min(node.position[axis]);
                max[axis] = max[axis].max(node.position[axis] + node.size[axis]);
            }
        }
        for id in &member_charts {
            let chart = charts[id];
            geometry.push((chart.position, chart.size));
            for axis in 0..2 {
                min[axis] = min[axis].min(chart.position[axis]);
                max[axis] = max[axis].max(chart.position[axis] + chart.size[axis]);
            }
        }
        for id in &member_connectors {
            for point in &connectors[id].waypoints {
                geometry.push((*point, [0.; 2]));
                for axis in 0..2 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
        }
        for id in &member_drawings {
            for point in drawings[id].points() {
                geometry.push((point, [0.; 2]));
                for axis in 0..2 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
        }
        units.insert(
            *group.members.iter().min().unwrap(),
            Unit {
                nodes: member_nodes.into_iter().collect(),
                connectors: member_connectors,
                charts: member_charts,
                drawings: member_drawings,
                position: min,
                size: [max[0] - min[0], max[1] - min[1]],
                geometry,
            },
        );
    }
    units.into_values().collect()
}

pub fn preview(
    doc: &Document,
    selection: Option<&[u64]>,
    direction: Direction,
) -> Result<Preview, String> {
    preview_with_spacing(doc, selection, direction, Spacing::default())
}

pub fn preview_with_spacing(
    doc: &Document,
    selection: Option<&[u64]>,
    direction: Direction,
    spacing: Spacing,
) -> Result<Preview, String> {
    if [spacing.levels, spacing.blocks]
        .iter()
        .any(|gap| !gap.is_finite() || !(20. ..=300.).contains(gap))
    {
        return Err("Espacements invalides : utilisez des valeurs de 20 à 300 px.".into());
    }
    doc.validate()?;
    let view = doc.organization_view();
    let selection = selection.map(|ids| ids.iter().copied().collect::<BTreeSet<_>>());
    let units = placement_units(doc, &view, selection.as_ref());
    if units.is_empty() {
        return Err("Aucun bloc à organiser.".into());
    }
    if units.iter().map(|unit| unit.nodes.len()).sum::<usize>() > 500 {
        return Err("Maximum de 500 blocs par mise en page.".into());
    }
    let indices: BTreeMap<_, _> = units
        .iter()
        .enumerate()
        .flat_map(|(i, unit)| unit.nodes.iter().map(move |id| (*id, i)))
        .collect();
    let unit_members: BTreeSet<u64> = units
        .iter()
        .flat_map(|unit| {
            unit.nodes
                .iter()
                .chain(&unit.connectors)
                .chain(&unit.charts)
                .chain(&unit.drawings)
        })
        .copied()
        .collect();
    let mut graph = vec![vec![]; units.len()];
    let mut reverse = graph.clone();
    for edge in &doc.connectors {
        if !view.is_object_visible(edge.id) {
            continue;
        }
        if let (Some(&a), Some(&b)) = (indices.get(&edge.from), indices.get(&edge.to))
            && a != b
        {
            graph[a].push(b);
            reverse[b].push(a);
        }
    }
    // Iterative Kosaraju avoids recursion for long chains and preserves cycles.
    let mut seen = vec![false; units.len()];
    let mut order = vec![];
    for start in 0..units.len() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut stack = vec![(start, 0)];
        while let Some((v, child)) = stack.last_mut() {
            if *child < graph[*v].len() {
                let next = graph[*v][*child];
                *child += 1;
                if !seen[next] {
                    seen[next] = true;
                    stack.push((next, 0));
                }
            } else {
                let (v, _) = stack.pop().unwrap();
                order.push(v);
            }
        }
    }
    let mut component = vec![usize::MAX; units.len()];
    let mut count = 0;
    for &start in order.iter().rev() {
        if component[start] != usize::MAX {
            continue;
        }
        component[start] = count;
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            for &next in &reverse[v] {
                if component[next] == usize::MAX {
                    component[next] = count;
                    stack.push(next);
                }
            }
        }
        count += 1;
    }
    let mut edges = vec![BTreeSet::new(); count];
    let mut incoming = vec![0; count];
    for (a, children) in graph.iter().enumerate() {
        for &b in children {
            let (a, b) = (component[a], component[b]);
            if a != b && edges[a].insert(b) {
                incoming[b] += 1;
            }
        }
    }
    let mut ready: BTreeSet<_> = (0..count).filter(|i| incoming[*i] == 0).collect();
    let mut rank = vec![0; count];
    while let Some(a) = ready.pop_first() {
        for &b in &edges[a] {
            rank[b] = rank[b].max(rank[a] + 1);
            incoming[b] -= 1;
            if incoming[b] == 0 {
                ready.insert(b);
            }
        }
    }
    let node_rank: Vec<_> = component.iter().map(|c| rank[*c]).collect();
    let mut levels = vec![vec![]; count];
    for (node, &level) in node_rank.iter().enumerate() {
        levels[level].push(node);
    }
    let crossings_before = crossing_count(&levels, &node_rank, &graph);
    let (levels, crossings_after) = reduce_crossings(levels, &node_rank, &graph, &reverse);
    let placement_order: Vec<_> = levels.iter().flatten().copied().collect();
    let axis = usize::from(direction == Direction::Vertical);
    let cross = 1 - axis;
    let mut gaps = [spacing.blocks; 2];
    gaps[axis] = spacing.levels;
    let mut origins = vec![40.; count];
    for level in 0..count {
        let extent = levels[level]
            .iter()
            .map(|index| units[*index].placed_extent(axis, origins[level]))
            .fold(0., f32::max);
        if level + 1 < count {
            origins[level + 1] = origins[level] + extent + spacing.levels;
        }
    }
    let mut occupied: Vec<(Point, Point)> = doc
        .nodes
        .iter()
        .filter(|n| !indices.contains_key(&n.id) && view.is_object_visible(n.id))
        .map(|n| (n.position, n.size))
        .chain(
            doc.charts
                .iter()
                .filter(|c| view.is_object_visible(c.id) && !unit_members.contains(&c.id))
                .map(|c| (c.position, c.size)),
        )
        .collect();
    let mut next = doc.clone();
    let mut moved = 0;
    let mut connector_placements = BTreeMap::new();
    let mut cursors = vec![40.; levels.len()];
    for index in placement_order {
        let unit = &units[index];
        let mut position = [40.; 2];
        let level = node_rank[index];
        position[axis] = origins[level];
        position[cross] = cursors[level];
        let mut size;
        // Skip fixed obstacles and previous placements, with bounded monotone progress.
        loop {
            size = unit.placed_size(position);
            let after = occupied
                .iter()
                .filter(|(p, obstacle_size)| {
                    (0..2).all(|a| {
                        position[a] < p[a] + obstacle_size[a] + gaps[a]
                            && position[a] + size[a] + gaps[a] > p[a]
                    })
                })
                .map(|(p, size)| p[cross] + size[cross] + spacing.blocks)
                .fold(position[cross], f32::max);
            if after == position[cross] {
                break;
            }
            position[cross] = after;
            if position[cross] + size[cross] > 100_000. {
                return Err("La mise en page dépasse les dimensions autorisées.".into());
            }
        }
        if position
            .iter()
            .zip(size)
            .any(|(p, size)| p + size > 100_000.)
        {
            return Err("La mise en page dépasse les dimensions autorisées.".into());
        }
        if position != unit.position {
            for id in &unit.nodes {
                let node = next.node_mut(*id).unwrap();
                for (axis, coordinate) in position.iter().enumerate() {
                    node.position[axis] = translated_coordinate(
                        node.position[axis],
                        unit.position[axis],
                        *coordinate,
                    );
                }
                moved += 1;
            }
            for id in &unit.connectors {
                connector_placements.insert(*id, (unit.position, position));
            }
            for id in &unit.charts {
                let chart = next
                    .charts
                    .iter_mut()
                    .find(|c| c.id == *id)
                    .expect("chart member still present");
                for (axis, coordinate) in position.iter().enumerate() {
                    chart.position[axis] = translated_coordinate(
                        chart.position[axis],
                        unit.position[axis],
                        *coordinate,
                    );
                }
            }
            for id in &unit.drawings {
                let drawing = next
                    .drawings
                    .iter_mut()
                    .find(|d| d.id == *id)
                    .expect("drawing member still present");
                for point in [&mut drawing.start, &mut drawing.end] {
                    for (axis, coordinate) in position.iter().enumerate() {
                        point[axis] =
                            translated_coordinate(point[axis], unit.position[axis], *coordinate);
                    }
                }
            }
        }
        occupied.push((position, size));
        cursors[level] = position[cross] + size[cross] + spacing.blocks;
        for (a, coordinate) in position.iter().enumerate() {
            next.page_size[a] = next.page_size[a].max(coordinate + size[a] + 40.);
        }
    }
    for edge in &mut next.connectors {
        if let Some((origin, position)) = connector_placements.get(&edge.id) {
            for point in &mut edge.waypoints {
                for (axis, coordinate) in position.iter().enumerate() {
                    point[axis] = translated_coordinate(point[axis], origin[axis], *coordinate);
                }
            }
        }
    }
    next.validate()?;
    doc.check_locked_changes(&next)?;
    Ok(Preview {
        document: next,
        moved,
        fixed: doc.nodes.len() - moved,
        crossings_before,
        crossings_after,
    })
}

// Positions are ranks within a level, independent of coordinates and obstacles.
fn row_positions(levels: &[Vec<usize>], count: usize) -> Vec<usize> {
    let mut rows = vec![0; count];
    for level in levels {
        for (row, &node) in level.iter().enumerate() {
            rows[node] = row;
        }
    }
    rows
}

/// Count edge inversions with a Fenwick tree, batching shared sources so fans
/// are never counted as crossings. Long edges that span several levels are
/// kept at the source's row until their final segment reaches the target row.
/// O(S log S + S log V), where S is the number of level segments, including
/// those spanned by long edges. This estimates crossings, not routed geometry.
fn crossing_count(levels: &[Vec<usize>], rank: &[usize], graph: &[Vec<usize>]) -> u64 {
    let rows = row_positions(levels, graph.len());
    let mut edges = vec![vec![]; levels.len()];
    for (a, children) in graph.iter().enumerate() {
        for &b in children {
            let r1 = rank[a];
            let r2 = rank[b];
            if r2 <= r1 {
                continue;
            }
            if r2 == r1 + 1 {
                edges[r1].push((rows[a], rows[b]));
            } else {
                // Keep the intermediate stubs at the source row, but connect
                // the final segment to the actual target. Otherwise shared
                // targets can count as crossings and reversed long links do not.
                for target in &mut edges[r1..r2] {
                    target.push((rows[a], rows[a]));
                }
                edges[r2 - 1].last_mut().unwrap().1 = rows[b];
            }
        }
    }
    let mut crossings = 0;
    for mut edges in edges {
        edges.sort_unstable();
        let mut tree = vec![0_u64; graph.len() + 1];
        let mut processed = 0;
        let mut start = 0;
        while start < edges.len() {
            let end = start
                + edges[start..]
                    .iter()
                    .take_while(|e| e.0 == edges[start].0)
                    .count();
            for &(_, row) in &edges[start..end] {
                let mut index = row + 1;
                let mut prefix = 0;
                while index > 0 {
                    prefix += tree[index];
                    index &= index - 1;
                }
                crossings += processed - prefix;
            }
            for &(_, row) in &edges[start..end] {
                let mut index = row + 1;
                while index < tree.len() {
                    tree[index] += 1;
                    index += index & index.wrapping_neg();
                }
                processed += 1;
            }
            start = end;
        }
    }
    crossings
}

fn reduce_crossings(
    mut levels: Vec<Vec<usize>>,
    rank: &[usize],
    graph: &[Vec<usize>],
    reverse: &[Vec<usize>],
) -> (Vec<Vec<usize>>, u64) {
    let mut best_score = crossing_count(&levels, rank, graph);
    let mut best = levels.clone();
    // Eight bounded directional sweeps. Keep the original ID order on ties.
    for pass in 0..8 {
        if best_score == 0 {
            break;
        }
        let forward = pass % 2 == 0;
        let order: Vec<_> = if forward {
            (1..levels.len()).collect()
        } else {
            (0..levels.len().saturating_sub(1)).rev().collect()
        };
        for level in order {
            let rows = row_positions(&levels, graph.len());
            let mut centers = vec![(0_u64, 1_u64); graph.len()];
            for &node in &levels[level] {
                let neighbors = if forward {
                    &reverse[node]
                } else {
                    &graph[node]
                };
                let mut sum = 0;
                let mut count = 0;
                for &other in neighbors {
                    if (forward && rank[other] + 1 == level)
                        || (!forward && rank[other] == level + 1)
                    {
                        sum += rows[other] as u64;
                        count += 1;
                    }
                }
                centers[node] = if count == 0 {
                    (rows[node] as u64, 1)
                } else {
                    (sum, count)
                };
            }
            levels[level].sort_by(|a, b| {
                let (sa, ca) = centers[*a];
                let (sb, cb) = centers[*b];
                (sa * cb)
                    .cmp(&(sb * ca))
                    .then_with(|| rows[*a].cmp(&rows[*b]))
            });
        }
        let score = crossing_count(&levels, rank, graph);
        if score < best_score {
            best_score = score;
            best = levels.clone();
        }
    }
    (best, best_score)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn oracle(levels: &[Vec<usize>], rank: &[usize], graph: &[Vec<usize>]) -> u64 {
        let rows = row_positions(levels, graph.len());
        let mut segments = vec![vec![]; levels.len()];
        for (a, children) in graph.iter().enumerate() {
            for &b in children {
                let r1 = rank[a];
                let r2 = rank[b];
                if r2 <= r1 {
                    continue;
                }
                for (level, target) in segments.iter_mut().enumerate().take(r2).skip(r1) {
                    let end = if level + 1 == r2 { rows[b] } else { rows[a] };
                    target.push((rows[a], end));
                }
            }
        }
        let mut count = 0;
        for segment in segments {
            for (index, &(a, b)) in segment.iter().enumerate() {
                for &(c, d) in &segment[index + 1..] {
                    if (a < c && b > d) || (a > c && b < d) {
                        count += 1;
                    }
                }
            }
        }
        count
    }
    #[test]
    fn every_three_by_three_graph_matches_pairwise_oracle_and_never_regresses() {
        let ranks = [0, 0, 0, 1, 1, 1];
        let original = vec![vec![0, 1, 2], vec![3, 4, 5]];
        for mask in 0..512 {
            let mut graph = vec![vec![]; 6];
            let mut reverse = graph.clone();
            for (a, children) in graph.iter_mut().enumerate().take(3) {
                for (b, parents) in reverse.iter_mut().enumerate().skip(3) {
                    if mask & (1 << (a * 3 + b - 3)) != 0 {
                        children.push(b);
                        parents.push(a);
                    }
                }
            }
            let before = crossing_count(&original, &ranks, &graph);
            assert_eq!(before, oracle(&original, &ranks, &graph));
            let (after, score) = reduce_crossings(original.clone(), &ranks, &graph, &reverse);
            assert_eq!(score, oracle(&after, &ranks, &graph));
            assert!(score <= before);
            assert_eq!(
                (after, score),
                reduce_crossings(original.clone(), &ranks, &graph, &reverse)
            );
        }
    }
    #[test]
    fn shared_endpoints_parallel_links_and_long_edges_are_counted_correctly() {
        let levels = vec![vec![0, 1], vec![2, 3], vec![4]];
        let rank = [0, 0, 1, 1, 2];
        let graph = vec![vec![3, 3, 2, 4], vec![2, 3], vec![4], vec![4], vec![]];
        assert_eq!(crossing_count(&levels, &rank, &graph), 2);
        assert_eq!(
            crossing_count(&levels, &rank, &graph),
            oracle(&levels, &rank, &graph)
        );
    }

    #[test]
    fn long_links_connect_to_their_destination_and_do_not_cross_shared_targets() {
        let levels = vec![vec![0, 1], vec![2, 3, 4], vec![5, 6]];
        let rank = [0, 0, 1, 1, 1, 2, 2];
        let mut graph = vec![vec![]; 7];
        graph[1] = vec![5];
        graph[4] = vec![5];
        assert_eq!(crossing_count(&levels, &rank, &graph), 0);
        // Two long links with reversed destinations cross once.
        graph[4].clear();
        graph[0] = vec![6];
        assert_eq!(crossing_count(&levels, &rank, &graph), 1);
    }

    #[test]
    fn every_three_level_graph_matches_the_oracle_and_never_regresses() {
        let original = vec![vec![0, 1], vec![2, 3], vec![4, 5]];
        let ranks = [0, 0, 1, 1, 2, 2];
        let pairs: Vec<_> = (0..6)
            .flat_map(|a| (0..6).map(move |b| (a, b)))
            .filter(|&(a, b)| ranks[a] < ranks[b])
            .collect();
        for mask in 0..(1 << pairs.len()) {
            let mut graph = vec![vec![]; 6];
            let mut reverse = graph.clone();
            for (bit, &(a, b)) in pairs.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    graph[a].push(b);
                    reverse[b].push(a);
                }
            }
            let before = crossing_count(&original, &ranks, &graph);
            assert_eq!(before, oracle(&original, &ranks, &graph));
            let (after, score) = reduce_crossings(original.clone(), &ranks, &graph, &reverse);
            assert_eq!(score, oracle(&after, &ranks, &graph));
            assert!(score <= before);
            assert_eq!(
                (after, score),
                reduce_crossings(original.clone(), &ranks, &graph, &reverse)
            );
        }
    }
}
