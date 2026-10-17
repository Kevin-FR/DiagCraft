//! Placement helpers stay outside the document and its serialized format.
use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) struct Guide {
    axis: usize,
    at: f32,
    span: [f32; 2],
}

fn bounds(position: Point, size: Point) -> Rect {
    Rect::from_min_size(pos2(position[0], position[1]), vec2(size[0], size[1]))
}

impl DrawApp {
    pub(super) fn prepare_alignment_capture(&mut self, guides: bool) {
        let mut doc = Document {
            title: "Placer les idées".into(),
            page_size: [1200., 800.],
            ..Document::default()
        };
        let mut ids = vec![];
        for (position, kind, text) in [
            ([180., 310.], ShapeKind::Rounded, "Déplacer un bloc"),
            ([600., 160.], ShapeKind::Rectangle, "Même colonne"),
            ([600., 490.], ShapeKind::Diamond, "Une décision"),
        ] {
            let id = doc.add_node(kind, position);
            let n = doc.node_mut(id).unwrap();
            n.position = position;
            n.size = [200., 100.];
            n.text = text.into();
            ids.push(id);
        }
        let c = doc.add_chart(ChartKind::Columns, [310., 620.]);
        let chart = doc.charts.iter_mut().find(|chart| chart.id == c).unwrap();
        chart.position = [180., 530.];
        chart.size = [260., 180.];
        self.replace(doc, None);
        self.show_library = false;
        self.show_inspector = !guides;
        self.show_quick_toolbar = false;
        self.ribbon_expanded = Some(false);
        self.selected = if guides {
            vec![ids[0]]
        } else {
            ids.into_iter().chain([c]).collect()
        };
        self.capture_alignment = guides;
        if guides {
            self.select_for_drag(self.selected[0], [180., 310.], false);
            let original = [(self.selected[0], [180., 310.])];
            let (delta, lines) = self.move_placement(&original, &[], [417., 0.], false);
            self.doc.node_mut(self.selected[0]).unwrap().position =
                [180. + delta[0], 310. + delta[1]];
            self.move_guides = lines;
            self.drag_snapshot = None;
        }
        self.fit_next = true;
    }

    fn placement_objects(&self) -> impl Iterator<Item = (u64, Rect)> + '_ {
        self.doc
            .nodes
            .iter()
            .map(|n| (n.id, bounds(n.position, n.size)))
            .chain(
                self.doc
                    .charts
                    .iter()
                    .map(|c| (c.id, bounds(c.position, c.size))),
            )
    }

    pub(super) fn placement_selection(&self) -> Vec<(u64, Rect)> {
        self.placement_objects()
            .filter(|(id, _)| self.selected.contains(id))
            .collect()
    }

    // Solve one translation for the whole group, including the grid, guides and
    // page limits. Independent per-object clamping would compress a selection.
    pub(super) fn move_placement(
        &self,
        original: &[(u64, Point)],
        charts: &[(u64, Point)],
        raw: Point,
        free: bool,
    ) -> (Point, Vec<Guide>) {
        let rects: Vec<_> = original
            .iter()
            .filter_map(|(id, p)| self.doc.node(*id).map(|n| bounds(*p, n.size)))
            .chain(charts.iter().filter_map(|(id, p)| {
                self.doc
                    .charts
                    .iter()
                    .find(|c| c.id == *id)
                    .map(|c| bounds(*p, c.size))
            }))
            .collect();
        let group = rects.iter().copied().reduce(Rect::union);
        let mut delta = raw;
        if self.snap && !free {
            let step = self.doc.page_style.grid_spacing / 2.;
            delta = raw.map(|v| (v / step).round() * step);
        }
        let mut guides = vec![];
        if self.snap
            && self.smart_guides
            && !free
            && raw[0].hypot(raw[1]) * self.zoom >= 3.
            && let Some(group) = group
        {
            let page = bounds([0., 0.], self.doc.page_size);
            let moving = group.translate(vec2(raw[0], raw[1]));
            let tolerance = 6. / self.zoom;
            // One closest target per axis; stable iteration resolves exact ties.
            let mut best: [Option<(f32, Guide)>; 2] = [None, None];
            for (_, peer) in self
                .placement_objects()
                .filter(|(id, r)| !self.selected.contains(id) && page.intersects(*r))
            {
                for (axis, closest) in best.iter_mut().enumerate() {
                    for (source, target) in [(0., 0.), (0.5, 0.5), (1., 1.), (0., 1.), (1., 0.)] {
                        let at = peer.min[axis] + peer.size()[axis] * target;
                        let correction = at - (moving.min[axis] + moving.size()[axis] * source);
                        if correction.abs() <= tolerance
                            && closest.is_none_or(|(old, _)| correction.abs() < old.abs())
                        {
                            let other = 1 - axis;
                            *closest = Some((
                                correction,
                                Guide {
                                    axis,
                                    at,
                                    span: [
                                        moving.min[other].min(peer.min[other]),
                                        moving.max[other].max(peer.max[other]),
                                    ],
                                },
                            ));
                        }
                    }
                }
            }
            for (axis, closest) in best.into_iter().enumerate() {
                if let Some((correction, guide)) = closest {
                    delta[axis] = raw[axis] + correction;
                    guides.push(guide);
                }
            }
        }
        let before_clamp = delta;
        if let Some(group) = group {
            let page = self.doc.edit_bounds();
            for axis in 0..2 {
                let low = -group.min[axis];
                let high = rects
                    .iter()
                    .map(|r| (page[axis] - r.size()[axis]).max(0.) - r.min[axis])
                    .reduce(f32::min)
                    .unwrap();
                if low <= high {
                    delta[axis] = delta[axis].clamp(low, high);
                }
            }
        }
        // A page limit must never leave an apparent alignment that was not met.
        guides.retain(|g| (before_clamp[g.axis] - delta[g.axis]).abs() < 0.001);
        // The perpendicular snap may have changed the group's span too.
        for g in &mut guides {
            let other = 1 - g.axis;
            if let Some(group) = group {
                g.span[0] = g.span[0].min(group.min[other] + delta[other]);
                g.span[1] = g.span[1].max(group.max[other] + delta[other]);
            }
        }
        (delta, guides)
    }

    pub(super) fn paint_alignment_guides(&self, p: &egui::Painter) {
        if !matches!(self.drag, Some(Drag::Move { .. })) || self.move_guides.is_empty() {
            return;
        }
        let color = Color32::from_rgb(196, 65, 145);
        for g in &self.move_guides {
            let point = |v| {
                let mut p = [0.; 2];
                p[g.axis] = g.at;
                p[1 - g.axis] = v;
                self.screen(p)
            };
            let a = point(g.span[0])
                - if g.axis == 0 {
                    vec2(0., 8.)
                } else {
                    vec2(8., 0.)
                };
            let b = point(g.span[1])
                + if g.axis == 0 {
                    vec2(0., 8.)
                } else {
                    vec2(8., 0.)
                };
            p.extend(Shape::dashed_line(&[a, b], Stroke::new(1., color), 4., 3.));
            for at in [point(g.span[0]), point(g.span[1])] {
                p.line_segment(
                    [at - vec2(3., 3.), at + vec2(3., 3.)],
                    Stroke::new(1., color),
                );
                p.line_segment(
                    [at - vec2(3., -3.), at + vec2(3., -3.)],
                    Stroke::new(1., color),
                );
            }
        }
    }

    pub(super) fn alignment_controls(&mut self, ui: &mut egui::Ui) {
        let count = self.placement_selection().len();
        ui.label(RichText::new("Aligner et répartir").strong());
        let button_width = ((ui.available_width() - 8.) / 3.).max(48.);
        egui::Grid::new("alignment_controls")
            .num_columns(3)
            .spacing(vec2(4., 4.))
            .show(ui, |ui| {
                for (i, (label, axis, anchor, hint)) in [
                    ("Gauche", 0, 0., "Aligner les bords gauches"),
                    ("Centre H", 0, 0.5, "Aligner les centres horizontalement"),
                    ("Droite", 0, 1., "Aligner les bords droits"),
                    ("Haut", 1, 0., "Aligner les bords supérieurs"),
                    ("Centre V", 1, 0.5, "Aligner les centres verticalement"),
                    ("Bas", 1, 1., "Aligner les bords inférieurs"),
                ]
                .into_iter()
                .enumerate()
                {
                    if ui
                        .add_enabled(
                            count >= 2,
                            egui::Button::new(RichText::new(label).size(12.))
                                .min_size(vec2(button_width, 28.)),
                        )
                        .on_hover_text(hint)
                        .clicked()
                    {
                        self.align_selected(axis, anchor);
                    }
                    if i % 3 == 2 {
                        ui.end_row();
                    }
                }
            });
        ui.horizontal(|ui| {
            for (label, vertical) in [("Espacer H", false), ("Espacer V", true)] {
                if ui.add_enabled(count >= 3, egui::Button::new(label)).on_hover_text("Espaces égaux entre les objets ; le premier et le dernier restent en place. Au moins trois formes, textes ou graphiques.").clicked() { self.distribute(vertical); }
            }
        });
        ui.label(RichText::new(format!("{count} formes, textes ou graphiques")).size(12.).color(MUTED))
            .on_hover_text("Les liens et tracés libres ne sont pas déplacés par ces commandes. Les liens attachés suivent leurs blocs.");
    }

    fn set_placement_axis(&mut self, id: u64, axis: usize, position: f32) {
        if let Some(n) = self.doc.node_mut(id) {
            n.position[axis] = position;
        } else if let Some(c) = self.doc.charts.iter_mut().find(|c| c.id == id) {
            c.position[axis] = position;
        }
    }

    pub(super) fn align_selected(&mut self, axis: usize, anchor: f32) {
        let objects = self.placement_selection();
        if objects.len() < 2 {
            return;
        }
        let group = objects.iter().map(|(_, r)| *r).reduce(Rect::union).unwrap();
        let target = group.min[axis] + group.size()[axis] * anchor;
        for (id, r) in &objects {
            self.set_placement_axis(*id, axis, target - r.size()[axis] * anchor);
        }
        self.commit();
        self.status = format!("{} objets alignés · ⌘Z pour annuler", objects.len());
    }

    pub(super) fn distribute(&mut self, vertical: bool) {
        let axis = usize::from(vertical);
        let mut objects = self.placement_selection();
        if objects.len() < 3 {
            return;
        }
        objects.sort_by(|a, b| a.1.min[axis].total_cmp(&b.1.min[axis]));
        let end = objects.last().unwrap().1.max[axis];
        let total: f32 = objects.iter().map(|(_, r)| r.size()[axis]).sum();
        let gap = (end - objects[0].1.min[axis] - total) / (objects.len() - 1) as f32;
        let mut at = objects[0].1.min[axis];
        // Keep both outer objects exactly fixed (no accumulated rounding drift).
        for (id, r) in objects.iter().take(objects.len() - 1) {
            self.set_placement_axis(*id, axis, at);
            at += r.size()[axis] + gap;
        }
        self.commit();
        self.status = format!("{} objets répartis · ⌘Z pour annuler", objects.len());
    }
}
