//! Optional page-local organization. Object IDs remain independent of group/layer IDs.
use crate::document::Document;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub const MAX_LAYERS: usize = 100;
pub const MAX_GROUPS: usize = 5_000;
fn yes() -> bool {
    true
}
fn is_false(value: &bool) -> bool {
    !*value
}
fn is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    #[serde(default = "yes")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub locked: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectGroup {
    pub id: u64,
    pub members: Vec<u64>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ObjectOrganization {
    #[serde(skip_serializing_if = "is_zero")]
    pub layer: u64,
    #[serde(skip_serializing_if = "is_false")]
    pub locked: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Organization {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<Layer>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ObjectGroup>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub objects: BTreeMap<u64, ObjectOrganization>,
}
impl Organization {
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty() && self.groups.is_empty() && self.objects.is_empty()
    }
}

/// Transient index for one page/frame. Never serialized or shared across edits.
#[derive(Clone, Debug)]
pub struct OrganizationView {
    pub visible_ids: HashSet<u64>,
    pub locked_ids: HashSet<u64>,
    pub editable_ids: HashSet<u64>,
    layers: HashMap<u64, u64>,
    layer_order: HashMap<u64, usize>,
    group_for_member: HashMap<u64, u64>,
    groups: Vec<Vec<u64>>,
}
impl OrganizationView {
    pub fn is_object_visible(&self, id: u64) -> bool {
        self.visible_ids.contains(&id)
    }
    pub fn is_object_locked(&self, id: u64) -> bool {
        self.locked_ids.contains(&id)
    }
    pub fn can_edit_object(&self, id: u64) -> bool {
        self.editable_ids.contains(&id)
    }
    pub fn object_layer(&self, id: u64) -> u64 {
        self.layers.get(&id).copied().unwrap_or(0)
    }
    pub fn layer_order(&self, id: u64) -> usize {
        self.layer_order.get(&id).copied().unwrap_or(0)
    }
    pub fn group_for_object(&self, id: u64) -> Option<u64> {
        self.group_for_member.get(&id).copied()
    }
    pub fn expand_groups(&self, ids: &[u64]) -> Vec<u64> {
        let mut selected: BTreeSet<_> = ids
            .iter()
            .copied()
            .filter(|id| self.visible_ids.contains(id))
            .collect();
        for group in &self.groups {
            if group.iter().any(|id| selected.contains(id))
                && group.iter().all(|id| self.visible_ids.contains(id))
            {
                selected.extend(group.iter().copied());
            }
        }
        selected.into_iter().collect()
    }
}

fn next_id(ids: impl Iterator<Item = u64>) -> u64 {
    let used: BTreeSet<_> = ids.collect();
    if let Some(id) = used.last().copied().unwrap_or(0).checked_add(1)
        && id < u64::MAX
    {
        return id;
    }
    (1..u64::MAX)
        .find(|id| !used.contains(id))
        .expect("bounded organization IDs")
}

impl Document {
    pub fn organization_view(&self) -> OrganizationView {
        let ids: HashSet<_> = self.object_ids().into_iter().collect();
        let layer_flags: HashMap<_, _> = std::iter::once((0, (true, false)))
            .chain(
                self.organization
                    .layers
                    .iter()
                    .map(|l| (l.id, (l.visible, l.locked))),
            )
            .collect();
        let layers: HashMap<_, _> = self
            .organization
            .objects
            .iter()
            .map(|(id, m)| (*id, m.layer))
            .collect();
        let mut visible_ids = HashSet::with_capacity(ids.len());
        let mut locked_ids = HashSet::new();
        for id in &ids {
            let layer = layers.get(id).copied().unwrap_or(0);
            let (visible, locked) = layer_flags.get(&layer).copied().unwrap_or((false, true));
            if visible {
                visible_ids.insert(*id);
            }
            if locked || self.organization.objects.get(id).is_some_and(|m| m.locked) {
                locked_ids.insert(*id);
            }
        }
        for edge in &self.connectors {
            if (edge.from != 0 && !visible_ids.contains(&edge.from))
                || (edge.to != 0 && !visible_ids.contains(&edge.to))
            {
                visible_ids.remove(&edge.id);
            }
        }
        for group in &self.organization.groups {
            if group.members.iter().any(|id| locked_ids.contains(id)) {
                locked_ids.extend(group.members.iter().copied());
            }
        }
        let mut editable_ids: HashSet<_> = visible_ids.difference(&locked_ids).copied().collect();
        for group in &self.organization.groups {
            if group.members.iter().any(|id| !visible_ids.contains(id)) {
                for id in &group.members {
                    editable_ids.remove(id);
                }
            }
        }
        OrganizationView {
            visible_ids,
            locked_ids,
            editable_ids,
            layers,
            layer_order: self
                .layer_ids()
                .into_iter()
                .enumerate()
                .map(|(i, id)| (id, i))
                .collect(),
            group_for_member: self
                .organization
                .groups
                .iter()
                .flat_map(|g| g.members.iter().map(move |id| (*id, g.id)))
                .collect(),
            groups: self
                .organization
                .groups
                .iter()
                .map(|g| g.members.clone())
                .collect(),
        }
    }
    pub fn visible_object_ids(&self) -> HashSet<u64> {
        self.organization_view().visible_ids
    }
    pub fn editable_object_ids(&self) -> HashSet<u64> {
        self.organization_view().editable_ids
    }
    pub fn object_ids(&self) -> Vec<u64> {
        self.nodes
            .iter()
            .map(|x| x.id)
            .chain(self.connectors.iter().map(|x| x.id))
            .chain(self.drawings.iter().map(|x| x.id))
            .chain(self.charts.iter().map(|x| x.id))
            .collect()
    }
    pub fn object_exists(&self, id: u64) -> bool {
        self.nodes.iter().any(|x| x.id == id)
            || self.connectors.iter().any(|x| x.id == id)
            || self.drawings.iter().any(|x| x.id == id)
            || self.charts.iter().any(|x| x.id == id)
    }
    pub fn object_layer(&self, id: u64) -> u64 {
        self.organization.objects.get(&id).map_or(0, |m| m.layer)
    }
    /// Bottom to top, including the implicit base layer.
    pub fn layer_ids(&self) -> Vec<u64> {
        std::iter::once(0)
            .chain(self.organization.layers.iter().map(|l| l.id))
            .collect()
    }
    pub fn layer_exists(&self, id: u64) -> bool {
        id == 0 || self.organization.layers.iter().any(|l| l.id == id)
    }
    pub fn layer_order(&self, id: u64) -> usize {
        self.organization
            .layers
            .iter()
            .position(|l| l.id == id)
            .map_or(0, |i| i + 1)
    }
    pub fn group_for_object(&self, id: u64) -> Option<&ObjectGroup> {
        self.organization
            .groups
            .iter()
            .find(|g| g.members.contains(&id))
    }
    pub fn is_object_visible(&self, id: u64) -> bool {
        self.organization_view().is_object_visible(id)
    }
    pub fn is_object_locked(&self, id: u64) -> bool {
        self.organization_view().is_object_locked(id)
    }
    pub fn can_edit_object(&self, id: u64) -> bool {
        self.organization_view().can_edit_object(id)
    }
    /// A hidden member must never be moved silently through its visible group.
    pub fn expand_groups(&self, ids: &[u64]) -> Vec<u64> {
        self.organization_view().expand_groups(ids)
    }
    pub fn validate_organization(&self) -> Result<(), String> {
        if self.organization.layers.len() >= MAX_LAYERS
            || self.organization.groups.len() > MAX_GROUPS
            || self.organization.objects.len() > 10_000
        {
            return Err("Organisation de page trop volumineuse.".into());
        }
        let objects: HashSet<_> = self.object_ids().into_iter().collect();
        let mut layers = HashSet::from([0]);
        for layer in &self.organization.layers {
            if layer.id == 0
                || layer.id == u64::MAX
                || !layers.insert(layer.id)
                || layer.name.trim().is_empty()
                || layer.name.len() > 800
            {
                return Err("Calque invalide ou dupliqué.".into());
            }
        }
        if self
            .organization
            .objects
            .iter()
            .any(|(id, m)| !objects.contains(id) || !layers.contains(&m.layer))
        {
            return Err("Organisation liée à un objet ou calque absent.".into());
        }
        let mut groups = HashSet::new();
        let mut members = HashSet::new();
        for group in &self.organization.groups {
            if group.id == 0
                || group.id == u64::MAX
                || !groups.insert(group.id)
                || group.members.len() < 2
                || group.members.len() > 10_000
            {
                return Err("Groupe invalide ou dupliqué.".into());
            }
            let layer = self.object_layer(group.members[0]);
            if group.members.iter().any(|id| {
                !objects.contains(id) || !members.insert(*id) || self.object_layer(*id) != layer
            }) {
                return Err(
                    "Les membres du groupe doivent être uniques et sur le même calque.".into(),
                );
            }
        }
        Ok(())
    }
    pub(crate) fn requested_ids(&self, ids: &[u64], editable: bool) -> Result<Vec<u64>, String> {
        let objects: HashSet<_> = self.object_ids().into_iter().collect();
        let view = self.organization_view();
        if ids.is_empty() || ids.len() > 10_000 || ids.iter().any(|id| !objects.contains(id)) {
            return Err("Sélection d’objets invalide.".into());
        }
        if editable && ids.iter().any(|id| !view.can_edit_object(*id)) {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        let mut expanded: BTreeSet<_> = ids.iter().copied().collect();
        for group in &self.organization.groups {
            if group.members.iter().any(|id| expanded.contains(id)) {
                expanded.extend(group.members.iter().copied());
            }
        }
        if editable && expanded.iter().any(|id| !view.can_edit_object(*id)) {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        Ok(expanded.into_iter().collect())
    }
    pub fn group_objects(&mut self, ids: &[u64]) -> Result<u64, String> {
        self.validate()?;
        let mut members = self.requested_ids(ids, true)?;
        let layer = self.object_layer(members[0]);
        if members.len() < 2 || members.iter().any(|id| self.object_layer(*id) != layer) {
            return Err("Sélectionnez au moins deux objets du même calque.".into());
        }
        let view = self.organization_view();
        let mut member_set: HashSet<_> = members.iter().copied().collect();
        for e in &self.connectors {
            if e.from != 0
                && e.to != 0
                && member_set.contains(&e.from)
                && member_set.contains(&e.to)
                && self.object_layer(e.id) == layer
                && view.can_edit_object(e.id)
                && !member_set.contains(&e.id)
            {
                members.push(e.id);
                member_set.insert(e.id);
            }
        }
        members = self.requested_ids(&members, true)?;
        if members.iter().any(|id| self.object_layer(*id) != layer) {
            return Err("Sélectionnez au moins deux objets du même calque.".into());
        }
        let member_set: HashSet<_> = members.iter().copied().collect();
        let retained: Vec<_> = self
            .organization
            .groups
            .iter()
            .filter(|g| !g.members.iter().any(|id| member_set.contains(id)))
            .cloned()
            .collect();
        if retained.len() >= MAX_GROUPS {
            return Err("Organisation de page trop volumineuse.".into());
        }
        let id = next_id(self.organization.groups.iter().map(|g| g.id));
        self.organization.groups = retained;
        self.organization.groups.push(ObjectGroup { id, members });
        Ok(id)
    }
    pub fn ungroup_objects(&mut self, ids: &[u64]) -> Result<(), String> {
        self.validate()?;
        let ids = self.requested_ids(ids, true)?;
        self.organization
            .groups
            .retain(|g| !g.members.iter().any(|id| ids.contains(id)));
        Ok(())
    }
    pub fn set_objects_locked(&mut self, ids: &[u64], locked: bool) -> Result<(), String> {
        self.validate()?;
        for id in self.requested_ids(ids, false)? {
            self.organization.objects.entry(id).or_default().locked = locked;
        }
        self.prune_organization();
        Ok(())
    }
    pub fn create_layer(&mut self, name: String) -> Result<u64, String> {
        self.validate()?;
        if self.organization.layers.len() + 1 >= MAX_LAYERS {
            return Err("Organisation de page trop volumineuse.".into());
        }
        let name = name.trim().to_owned();
        if name.is_empty() || name.len() > 800 {
            return Err("Nom de calque invalide.".into());
        }
        let id = next_id(self.organization.layers.iter().map(|l| l.id));
        self.organization.layers.push(Layer {
            id,
            name,
            visible: true,
            locked: false,
        });
        Ok(id)
    }
    pub fn update_layer(
        &mut self,
        id: u64,
        name: Option<String>,
        visible: Option<bool>,
        locked: Option<bool>,
    ) -> Result<(), String> {
        self.validate()?;
        let index = self
            .organization
            .layers
            .iter()
            .position(|l| l.id == id)
            .ok_or("Calque introuvable ou réservé.")?;
        let name = name.map(|name| name.trim().to_owned());
        if name
            .as_ref()
            .is_some_and(|name| name.is_empty() || name.len() > 800)
        {
            return Err("Nom de calque invalide.".into());
        }
        let layer = &mut self.organization.layers[index];
        if let Some(name) = name {
            layer.name = name;
        }
        if let Some(visible) = visible {
            layer.visible = visible;
        }
        if let Some(locked) = locked {
            layer.locked = locked;
        }
        Ok(())
    }
    /// Index among explicit layers, zero is immediately above the base layer.
    pub fn move_layer(&mut self, id: u64, index: usize) -> Result<(), String> {
        self.validate()?;
        if index >= self.organization.layers.len() {
            return Err("Position de calque invalide.".into());
        }
        let from = self
            .organization
            .layers
            .iter()
            .position(|l| l.id == id)
            .ok_or("Calque introuvable ou réservé.")?;
        let layer = self.organization.layers.remove(from);
        self.organization.layers.insert(index, layer);
        Ok(())
    }
    pub fn move_objects_to_layer(&mut self, ids: &[u64], layer: u64) -> Result<(), String> {
        self.validate()?;
        if !self.layer_exists(layer) {
            return Err("Calque introuvable ou réservé.".into());
        }
        if self
            .organization
            .layers
            .iter()
            .any(|l| l.id == layer && (!l.visible || l.locked))
        {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        let ids = self.requested_ids(ids, true)?;
        for id in ids {
            self.organization.objects.entry(id).or_default().layer = layer;
        }
        self.prune_organization();
        Ok(())
    }
    pub fn remove_layer(&mut self, id: u64) -> Result<(), String> {
        self.validate()?;
        let index = self
            .organization
            .layers
            .iter()
            .position(|l| l.id == id)
            .ok_or("Calque introuvable ou réservé.")?;
        // Layer deletion moves its objects to base; it does not delete content.
        self.organization.layers.remove(index);
        for item in self.organization.objects.values_mut() {
            if item.layer == id {
                item.layer = 0;
            }
        }
        self.prune_organization();
        Ok(())
    }
    pub fn prune_organization(&mut self) {
        let ids: HashSet<_> = self.object_ids().into_iter().collect();
        self.organization
            .objects
            .retain(|id, m| ids.contains(id) && (m.layer != 0 || m.locked));
        self.organization.groups.retain_mut(|group| {
            group.members.retain(|id| ids.contains(id));
            group.members.len() >= 2
        });
    }
    fn check_active_locked_changes(&self, after: &Self) -> Result<(), String> {
        let view = self.organization_view();
        let protected: Vec<_> = self
            .object_ids()
            .into_iter()
            .filter(|id| !view.can_edit_object(*id))
            .collect();
        if protected.is_empty() {
            return Ok(());
        }
        let nodes: HashMap<_, _> = after.nodes.iter().map(|n| (n.id, n)).collect();
        let edges: HashMap<_, _> = after.connectors.iter().map(|n| (n.id, n)).collect();
        let drawings: HashMap<_, _> = after.drawings.iter().map(|n| (n.id, n)).collect();
        let charts: HashMap<_, _> = after.charts.iter().map(|n| (n.id, n)).collect();
        let protected: HashSet<_> = protected.into_iter().collect();
        let changed = self
            .nodes
            .iter()
            .any(|n| protected.contains(&n.id) && nodes.get(&n.id).copied() != Some(n))
            || self
                .connectors
                .iter()
                .any(|n| protected.contains(&n.id) && edges.get(&n.id).copied() != Some(n))
            || self
                .drawings
                .iter()
                .any(|n| protected.contains(&n.id) && drawings.get(&n.id).copied() != Some(n))
            || self
                .charts
                .iter()
                .any(|n| protected.contains(&n.id) && charts.get(&n.id).copied() != Some(n));
        if changed {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        Ok(())
    }
    /// Defense against accidental direct field writes. Metadata changes alone
    /// are permitted; unlocking and editing must be separate transitions.
    pub fn check_locked_changes(&self, after: &Self) -> Result<(), String> {
        if self.organization.is_empty() && self.pages.iter().all(|p| p.organization.is_empty()) {
            return Ok(());
        }
        let before_pages: Vec<_> = (0..self.page_count())
            .map(|p| self.page(p).unwrap())
            .collect();
        let after_pages: Vec<_> = (0..after.page_count())
            .map(|p| after.page(p).unwrap())
            .collect();
        if before_pages.len() == after_pages.len() {
            // An exact permutation is a page reorder, not an object edit.
            let mut unmatched = after_pages.clone();
            let permutation = before_pages.iter().all(|page| {
                if let Some(index) = unmatched.iter().position(|p| p == page) {
                    unmatched.remove(index);
                    true
                } else {
                    false
                }
            });
            if permutation {
                return Ok(());
            }
            for (before, after) in before_pages.into_iter().zip(after_pages) {
                Self::from_page(before, "")
                    .check_active_locked_changes(&Self::from_page(after, ""))?;
            }
        } else {
            let mut unmatched = after_pages;
            for page in before_pages {
                if let Some(index) = unmatched.iter().position(|p| p == &page) {
                    unmatched.remove(index);
                } else {
                    let before = Self::from_page(page, "");
                    if before.organization_view().editable_ids.len() < before.object_ids().len() {
                        return Err("Déverrouillez les objets et affichez leur calque avant de les modifier.".into());
                    }
                }
            }
        }
        Ok(())
    }
    pub(crate) fn selected_organization(&self, ids: &HashSet<u64>) -> Organization {
        let objects: BTreeMap<_, _> = self
            .organization
            .objects
            .iter()
            .filter(|(id, _)| ids.contains(id))
            .map(|(id, m)| (*id, m.clone()))
            .collect();
        let layers = self
            .organization
            .layers
            .iter()
            .filter(|l| objects.values().any(|m| m.layer == l.id))
            .cloned()
            .collect();
        let groups = self
            .organization
            .groups
            .iter()
            .filter(|g| g.members.iter().all(|id| ids.contains(id)))
            .cloned()
            .collect();
        Organization {
            layers,
            groups,
            objects,
        }
    }
    pub(crate) fn paste_organization(
        &mut self,
        source: &Self,
        ids: &HashMap<u64, u64>,
        reuse_layers: bool,
    ) -> Result<(), String> {
        let mut layers = HashMap::from([(0, 0)]);
        for old in &source.organization.layers {
            if reuse_layers && self.layer_exists(old.id) {
                layers.insert(old.id, old.id);
                continue;
            }
            let id = self.create_layer(old.name.clone())?;
            self.update_layer(id, None, Some(old.visible), Some(false))?;
            layers.insert(old.id, id);
        }
        for (old, metadata) in &source.organization.objects {
            self.organization.objects.insert(
                ids[old],
                ObjectOrganization {
                    layer: layers[&metadata.layer],
                    locked: false,
                },
            );
        }
        for group in &source.organization.groups {
            if self.organization.groups.len() >= MAX_GROUPS {
                return Err("Organisation de page trop volumineuse.".into());
            }
            let id = next_id(self.organization.groups.iter().map(|g| g.id));
            self.organization.groups.push(ObjectGroup {
                id,
                members: group.members.iter().map(|m| ids[m]).collect(),
            });
        }
        self.prune_organization();
        Ok(())
    }
}
