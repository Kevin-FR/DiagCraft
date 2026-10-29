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
    /// Straight centre-line crossings between consecutive levels, not routed geometry.
    // Baseline is the automatic ID order, not the source drawing's geometry.
    pub crossings_before: u64,
    pub crossings_after: u64,
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
    let nodes: BTreeMap<_, _> = doc
        .nodes
        .iter()
        .filter(|n| {
            n.kind != ShapeKind::Text
                && view.can_edit_object(n.id)
                && view.group_for_object(n.id).is_none()
                && selection.as_ref().is_none_or(|ids| ids.contains(&n.id))
        })
        .map(|n| (n.id, n))
        .collect();
    if nodes.is_empty() {
        return Err("Aucun bloc à organiser.".into());
    }
    if nodes.len() > 500 {
        return Err("Maximum de 500 blocs par mise en page.".into());
    }
    let indices: BTreeMap<_, _> = nodes.keys().enumerate().map(|(i, id)| (*id, i)).collect();
    let mut graph = vec![vec![]; nodes.len()];
    let mut reverse = graph.clone();
    for edge in &doc.connectors {
        if !view.is_object_visible(edge.id) {
            continue;
        }
        if let (Some(&a), Some(&b)) = (indices.get(&edge.from), indices.get(&edge.to)) {
            graph[a].push(b);
            reverse[b].push(a);
        }
    }
    // Iterative Kosaraju avoids recursion for long chains and preserves cycles.
    let mut seen = vec![false; nodes.len()];
    let mut order = vec![];
    for start in 0..nodes.len() {
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
    let mut component = vec![usize::MAX; nodes.len()];
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
    let ids: Vec<_> = nodes.keys().copied().collect();
    let placement_order: Vec<_> = levels.iter().flatten().map(|index| ids[*index]).collect();
    let axis = usize::from(direction == Direction::Vertical);
    let cross = 1 - axis;
    let mut gaps = [spacing.blocks; 2];
    gaps[axis] = spacing.levels;
    let mut extents = vec![0_f32; count];
    for (&id, node) in &nodes {
        let r = rank[component[indices[&id]]];
        extents[r] = extents[r].max(node.size[axis]);
    }
    let mut origins = vec![40.; count];
    for r in 1..count {
        origins[r] = origins[r - 1] + extents[r - 1] + spacing.levels;
    }
    let mut occupied: Vec<(Point, Point)> = doc
        .nodes
        .iter()
        .filter(|n| !nodes.contains_key(&n.id) && view.is_object_visible(n.id))
        .map(|n| (n.position, n.size))
        .chain(
            doc.charts
                .iter()
                .filter(|c| view.is_object_visible(c.id))
                .map(|c| (c.position, c.size)),
        )
        .collect();
    let mut next = doc.clone();
    let mut moved = 0;
    let mut cursors = vec![40.; levels.len()];
    for id in placement_order {
        let node = nodes[&id];
        let mut position = [40.; 2];
        let level = node_rank[indices[&id]];
        position[axis] = origins[level];
        position[cross] = cursors[level];
        // Skip fixed obstacles and previous placements, with bounded monotone progress.
        loop {
            let after = occupied
                .iter()
                .filter(|(p, size)| {
                    (0..2).all(|a| {
                        position[a] < p[a] + size[a] + gaps[a]
                            && position[a] + node.size[a] + gaps[a] > p[a]
                    })
                })
                .map(|(p, size)| p[cross] + size[cross] + spacing.blocks)
                .fold(position[cross], f32::max);
            if after == position[cross] {
                break;
            }
            position[cross] = after;
            if position[cross] + node.size[cross] > 100_000. {
                return Err("La mise en page dépasse les dimensions autorisées.".into());
            }
        }
        if position
            .iter()
            .zip(node.size)
            .any(|(p, size)| p + size > 100_000.)
        {
            return Err("La mise en page dépasse les dimensions autorisées.".into());
        }
        if position != node.position {
            moved += 1;
        }
        next.node_mut(id).unwrap().position = position;
        occupied.push((position, node.size));
        cursors[level] = position[cross] + node.size[cross] + spacing.blocks;
        for (a, coordinate) in position.iter().enumerate() {
            next.page_size[a] = next.page_size[a].max(coordinate + node.size[a] + 40.);
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
/// are never counted as crossings. O(E log E + E log V), including dense graphs.
fn crossing_count(levels: &[Vec<usize>], rank: &[usize], graph: &[Vec<usize>]) -> u64 {
    let rows = row_positions(levels, graph.len());
    let mut edges = vec![vec![]; levels.len()];
    for (a, children) in graph.iter().enumerate() {
        for &b in children {
            if rank[b] == rank[a] + 1 {
                edges[rank[a]].push((rows[a], rows[b]));
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
        let edges: Vec<_> = graph
            .iter()
            .enumerate()
            .flat_map(|(a, children)| children.iter().map(move |&b| (a, b)))
            .filter(|(a, b)| rank[*b] == rank[*a] + 1)
            .collect();
        let mut count = 0;
        for (index, &(a, b)) in edges.iter().enumerate() {
            for &(c, d) in &edges[index + 1..] {
                if rank[a] == rank[c]
                    && ((rows[a] < rows[c] && rows[b] > rows[d])
                        || (rows[a] > rows[c] && rows[b] < rows[d]))
                {
                    count += 1;
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
}
