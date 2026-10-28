//! View-only selection gestures: no document edits or history snapshots.
use super::*;

fn crosses(points: &[Point], rect: Rect, closed: bool) -> bool {
    let segment = |a: Point, b: Point| {
        interaction_ui::clip_interval(pos2(a[0], a[1]), vec2(b[0] - a[0], b[1] - a[1]), rect)
            .is_some()
    };
    points.iter().any(|p| rect.contains(pos2(p[0], p[1])))
        || points.windows(2).any(|p| segment(p[0], p[1]))
        || (closed && points.len() > 2 && segment(points[points.len() - 1], points[0]))
}
fn node_crosses(n: &Node, rect: Rect) -> bool {
    let bounds = Rect::from_min_size(
        pos2(n.position[0], n.position[1]),
        vec2(n.size[0], n.size[1]),
    );
    if !bounds.intersects(rect) {
        return false;
    }
    if n.kind == ShapeKind::Ellipse {
        let nearest = rect.clamp(pos2(n.center()[0], n.center()[1]));
        return n.contains([nearest.x, nearest.y]);
    }
    if rect.contains(pos2(n.center()[0], n.center()[1]))
        || [
            rect.left_top(),
            rect.right_top(),
            rect.left_bottom(),
            rect.right_bottom(),
        ]
        .into_iter()
        .any(|p| n.contains([p.x, p.y]))
    {
        return true;
    }
    let shape = geometry::shape(n.kind);
    let transform = |points: &[Point]| {
        points
            .iter()
            .map(|p| {
                [
                    n.position[0] + p[0] * n.size[0],
                    n.position[1] + p[1] * n.size[1],
                ]
            })
            .collect::<Vec<_>>()
    };
    crosses(&transform(&shape.outline), rect, true)
        || shape
            .details
            .iter()
            .any(|p| crosses(&transform(p), rect, false))
        || shape
            .fills
            .iter()
            .any(|p| crosses(&transform(p), rect, true))
}
impl DrawApp {
    pub(super) fn marquee_hits(&self, start: Point, end: Point) -> Vec<u64> {
        let rect = Rect::from_two_pos(pos2(start[0], start[1]), pos2(end[0], end[1])).intersect(
            Rect::from_min_size(
                Pos2::ZERO,
                vec2(self.doc.page_size[0], self.doc.page_size[1]),
            ),
        );
        if !rect.is_positive() {
            return vec![];
        }
        let routes = self.route_cache.borrow_mut().scene(&self.doc);
        let view = self.doc.organization_view();
        let hits = self
            .doc
            .nodes
            .iter()
            .filter(|n| view.is_object_visible(n.id) && node_crosses(n, rect))
            .map(|n| n.id)
            .chain(
                self.doc
                    .drawings
                    .iter()
                    .filter(|d| {
                        view.is_object_visible(d.id)
                            && crosses(&d.points(), rect.expand(d.width / 2.), false)
                    })
                    .map(|d| d.id),
            )
            .chain(
                self.doc
                    .charts
                    .iter()
                    .filter(|c| {
                        view.is_object_visible(c.id)
                            && rect.intersects(Rect::from_min_size(
                                pos2(c.position[0], c.position[1]),
                                vec2(c.size[0], c.size[1]),
                            ))
                    })
                    .map(|c| c.id),
            )
            .chain(
                self.doc
                    .connectors
                    .iter()
                    .filter(|e| {
                        view.is_object_visible(e.id)
                            && crosses(&routes[&e.id].points, rect.expand(e.width / 2.), false)
                    })
                    .map(|e| e.id),
            )
            .collect::<Vec<_>>();
        view.expand_groups(&hits)
    }
    pub(super) fn cancel_marquee(&mut self) {
        if let Some(Drag::Marquee { initial, .. }) = self.drag.take() {
            self.selected = initial;
            self.status = "Sélection précédente restaurée".into();
        }
    }
    pub(super) fn update_marquee(&mut self, ui: &egui::Ui, pos: Option<Pos2>) -> bool {
        let Some(Drag::Marquee {
            start,
            current,
            initial,
            additive,
        }) = &self.drag
        else {
            return false;
        };
        let start = *start;
        let mut current = *current;
        let initial = initial.clone();
        let additive = *additive;
        if ui.input(|i| !i.focused) {
            self.cancel_marquee();
            return true;
        }
        if let Some(pos) = pos {
            current = self.world(self.canvas.clamp(pos));
        }
        let moved = self.screen(start).distance(self.screen(current)) >= 4.;
        if moved {
            let hits = self.marquee_hits(start, current);
            self.selected = if additive { initial } else { vec![] };
            for id in hits {
                if !self.selected.contains(&id) {
                    self.selected.push(id);
                }
            }
        }
        if let Some(Drag::Marquee {
            current: stored, ..
        }) = &mut self.drag
        {
            *stored = current;
        }
        if ui.input(|i| i.pointer.primary_released()) {
            if !moved && !additive {
                self.selected.clear();
            }
            self.drag = None;
            self.status = i18n::render(
                "Objets sélectionnés : {0} · Maj + F pour zoomer",
                &[("0", self.selected.len().to_string())],
            );
        }
        true
    }
    pub(super) fn paint_marquee(&self, p: &egui::Painter) {
        let Some(Drag::Marquee {
            start,
            current,
            additive,
            ..
        }) = &self.drag
        else {
            return;
        };
        if self.screen(*start).distance(self.screen(*current)) < 4. {
            return;
        }
        let rect =
            Rect::from_two_pos(self.screen(*start), self.screen(*current)).intersect(self.canvas);
        p.rect(
            rect,
            2,
            ACCENT.gamma_multiply(0.08),
            Stroke::new(1.5, ACCENT),
            StrokeKind::Inside,
        );
        let text = p.layout_no_wrap(
            i18n::render(
                "Sélection : {0}{1} · Échap : annuler",
                &[
                    ("0", self.selected.len().to_string()),
                    (
                        "1",
                        if *additive {
                            t(" · ajout")
                        } else {
                            String::new()
                        },
                    ),
                ],
            ),
            FontId::proportional(12.),
            INK,
        );
        let size = text.size() + vec2(16., 10.);
        let at = pos2(
            (rect.right() + 8.).clamp(
                self.canvas.left() + 4.,
                (self.canvas.right() - size.x - 4.).max(self.canvas.left() + 4.),
            ),
            (rect.bottom() + 8.).clamp(
                self.canvas.top() + 4.,
                (self.canvas.bottom() - size.y - 4.).max(self.canvas.top() + 4.),
            ),
        );
        p.rect(
            Rect::from_min_size(at, size),
            5,
            Color32::WHITE,
            Stroke::new(1., BORDER),
            StrokeKind::Inside,
        );
        p.galley(at + vec2(8., 5.), text, INK);
    }
    pub(super) fn fit_selection(&mut self, ctx: &egui::Context) {
        if self.drag.is_some() {
            return;
        }
        let mut bounds = Rect::NOTHING;
        for n in &self.doc.nodes {
            if self.selected.contains(&n.id) {
                let b = diagcraft::appearance_svg::node_bounds(n);
                bounds = bounds.union(Rect::from_min_max(
                    pos2(n.position[0] + b[0], n.position[1] + b[1]),
                    pos2(n.position[0] + b[2], n.position[1] + b[3]),
                ));
            }
        }
        for c in &self.doc.charts {
            if self.selected.contains(&c.id) {
                bounds = bounds.union(Rect::from_min_size(
                    pos2(c.position[0], c.position[1]),
                    vec2(c.size[0], c.size[1]),
                ));
            }
        }
        for e in &self.doc.connectors {
            if self.selected.contains(&e.id) {
                let mut r = Rect::NOTHING;
                for p in self.cached_route(e.id) {
                    r.extend_with(pos2(p[0], p[1]));
                }
                bounds = bounds.union(r.expand(
                    (8. + e.width) * e.appearance.start_scale.max(e.appearance.end_scale) * 1.5,
                ));
                if !e.label.is_empty() {
                    let size = ctx
                        .fonts_mut(|f| {
                            f.layout_no_wrap(
                                e.label.clone(),
                                FontId::proportional(14.),
                                Color32::BLACK,
                            )
                        })
                        .size()
                        + vec2(6., 6.);
                    let at = self.doc.edge_label_point(e);
                    bounds = bounds.union(Rect::from_center_size(pos2(at[0], at[1]), size));
                }
            }
        }
        for d in &self.doc.drawings {
            if self.selected.contains(&d.id) {
                let mut r = Rect::NOTHING;
                for p in d.points() {
                    r.extend_with(pos2(p[0], p[1]));
                }
                bounds = bounds.union(r.expand((8. + d.width) * d.appearance.end_scale * 1.5));
            }
        }
        if !bounds.is_finite() {
            self.status = "Sélectionnez un objet pour le cadrer".into();
            return;
        }
        let size = bounds.size().max(vec2(24., 24.));
        self.zoom = ((self.canvas.width() - 96.) / size.x)
            .min((self.canvas.height() - 96.) / size.y)
            .clamp(0.2, 2.5);
        self.pan = self.canvas.size() / 2. - bounds.center().to_vec2() * self.zoom;
        self.fit_next = false;
        self.status = "Vue centrée sur la sélection · F pour revoir la page".into();
    }
}
