//! Deterministic, bounded flow placement. Produces a new document for review.
use crate::document::{Document, Point, ShapeKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Horizontal,
    Vertical,
}
pub struct Preview {
    pub document: Document,
    pub moved: usize,
    pub fixed: usize,
}

pub fn preview(
    doc: &Document,
    selection: Option<&[u64]>,
    direction: Direction,
) -> Result<Preview, String> {
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
    let axis = usize::from(direction == Direction::Vertical);
    let cross = 1 - axis;
    let gap = 60.;
    let mut extents = vec![0_f32; count];
    for (&id, node) in &nodes {
        let r = rank[component[indices[&id]]];
        extents[r] = extents[r].max(node.size[axis]);
    }
    let mut origins = vec![40.; count];
    for r in 1..count {
        origins[r] = origins[r - 1] + extents[r - 1] + gap;
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
    for (&id, node) in &nodes {
        let mut position = [40.; 2];
        position[axis] = origins[rank[component[indices[&id]]]];
        // Skip fixed obstacles and previous placements, with bounded monotone progress.
        loop {
            let after = occupied
                .iter()
                .filter(|(p, size)| {
                    (0..2).all(|a| {
                        position[a] < p[a] + size[a] + gap
                            && position[a] + node.size[a] + gap > p[a]
                    })
                })
                .map(|(p, size)| p[cross] + size[cross] + gap)
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
    })
}
