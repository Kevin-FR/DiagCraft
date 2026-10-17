//! Reversible pointer gestures and constrained resizing; no persisted fields.
use super::*;

pub(super) struct DragSnapshot {
    document: Document,
    selection: Vec<u64>,
}

pub(super) struct ResizeSettings {
    pub bounds: Point,
    pub minimum: f32,
    pub grid: Option<f32>,
    pub proportional: bool,
}

pub(super) fn resize_geometry(
    origin: Point,
    size: Point,
    handle: [i8; 2],
    delta: Point,
    settings: ResizeSettings,
) -> (Point, Point) {
    let ResizeSettings {
        bounds,
        minimum,
        grid,
        proportional,
    } = settings;
    let mut position = origin;
    let mut extent = size;
    if proportional {
        let driver = if handle[0] == 0 {
            1
        } else if handle[1] == 0 || (delta[0] / size[0]).abs() >= (delta[1] / size[1]).abs() {
            0
        } else {
            1
        };
        let sign = f32::from(handle[driver]);
        let edge = origin[driver] + if sign > 0. { size[driver] } else { 0. };
        let mut at = edge + delta[driver];
        if let Some(step) = grid {
            at = (at / step).round() * step;
        }
        let mut scale = 1. + (at - edge) * sign / size[driver];
        let mut min_scale = (minimum / size[0]).max(minimum / size[1]);
        let mut max_scale = (20_000. / size[0]).min(20_000. / size[1]);
        for axis in 0..2 {
            let available = match handle[axis] {
                -1 => origin[axis] + size[axis],
                1 => bounds[axis] - origin[axis],
                _ => {
                    let center = origin[axis] + size[axis] / 2.;
                    2. * center.min(bounds[axis] - center)
                }
            };
            max_scale = max_scale.min(available / size[axis]);
            let anchor = match handle[axis] {
                -1 => 1.,
                1 => 0.,
                _ => 0.5,
            };
            if anchor > 0. {
                let span = anchor * size[axis];
                min_scale = min_scale.max((origin[axis] + span - 100_000.) / span);
                max_scale = max_scale.min((origin[axis] + span + 100_000.) / span);
            }
        }
        // Existing oversized/off-page objects may already make page constraints
        // impossible. Keep the opposite anchor and the object's valid ratio.
        scale = scale.clamp(min_scale, max_scale.max(min_scale));
        for axis in 0..2 {
            extent[axis] = (size[axis] * scale).clamp(minimum, 20_000.);
            position[axis] = origin[axis]
                + match handle[axis] {
                    -1 => size[axis] - extent[axis],
                    1 => 0.,
                    _ => (size[axis] - extent[axis]) / 2.,
                };
            position[axis] = position[axis].clamp(-100_000., 100_000.);
        }
        return (position, extent);
    }
    for axis in 0..2 {
        if handle[axis] == 0 {
            continue;
        }
        let mut at = origin[axis] + if handle[axis] < 0 { 0. } else { size[axis] } + delta[axis];
        if let Some(step) = grid {
            at = (at / step).round() * step;
        }
        if handle[axis] < 0 {
            let fixed = origin[axis] + size[axis];
            let at = at.clamp((fixed - 20_000.).max(0.), (fixed - minimum).max(0.));
            position[axis] = at;
            extent[axis] = fixed - at;
        } else {
            extent[axis] = (at - origin[axis]).clamp(
                minimum,
                (bounds[axis] - origin[axis]).clamp(minimum, 20_000.),
            );
        }
    }
    (position, extent)
}

fn resize_cursor(handle: [i8; 2]) -> egui::CursorIcon {
    match handle {
        [0, _] => egui::CursorIcon::ResizeVertical,
        [_, 0] => egui::CursorIcon::ResizeHorizontal,
        [x, y] if x == y => egui::CursorIcon::ResizeNwSe,
        _ => egui::CursorIcon::ResizeNeSw,
    }
}

impl DrawApp {
    pub(super) fn prepare_resize_capture(&mut self) {
        self.prepare_alignment_capture(false);
        self.doc.title = "Redimensionner avec précision".into();
        let id = self.doc.nodes[0].id;
        let origin = self.doc.nodes[0].position;
        let size = self.doc.nodes[0].size;
        self.doc.nodes[0].text = "Conserver les proportions".into();
        let (position, extent) = resize_geometry(
            origin,
            size,
            [1, 1],
            [120., 60.],
            ResizeSettings {
                bounds: self.doc.edit_bounds(),
                minimum: 24.,
                grid: None,
                proportional: true,
            },
        );
        self.doc.nodes[0].position = position;
        self.doc.nodes[0].size = extent;
        self.selected = vec![id];
        self.drag = Some(Drag::Resize {
            id,
            start: [origin[0] + size[0], origin[1] + size[1]],
            size,
            origin,
            handle: [1, 1],
        });
        self.drag_snapshot = None;
        self.show_inspector = false;
        self.capture_alignment = true;
    }

    pub(super) fn begin_edit_drag(&mut self) {
        // Flush previous edits without growing an automatic page just because
        // an object was pressed. Cancellation preserves the redo branch too.
        self.history.record(&self.doc);
        self.property_edit = false;
        self.drag_snapshot = Some(DragSnapshot {
            document: self.doc.clone(),
            selection: self.selected.clone(),
        });
    }

    pub(super) fn cancel_edit_drag(&mut self) {
        if let Some(before) = self.drag_snapshot.take() {
            self.doc = before.document;
            self.selected = before.selection;
            self.drag = None;
            self.move_guides.clear();
            self.status = "Geste annulé · les modifications précédentes sont conservées".into();
        }
    }

    pub(super) fn drag_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        if self.drag_snapshot.is_none() || self.drag.is_none() {
            return false;
        }
        if ctx.input(|i| {
            !i.focused
                || i.key_pressed(Key::Escape)
                || (i.modifiers.command && i.key_pressed(Key::Z))
        }) {
            self.cancel_edit_drag();
        } else if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::S)) {
            let save_as = ctx.input(|i| i.modifiers.shift);
            self.commit();
            self.save(save_as);
        }
        // Editing shortcuts cannot interleave document mutations with a drag.
        true
    }

    pub(super) fn resize_handle_at(&self, pos: Pos2) -> Option<(u64, Point, Point, [i8; 2])> {
        let id = *self.selected.first().filter(|_| self.selected.len() == 1)?;
        let (origin, size) = self
            .doc
            .node(id)
            .map(|n| (n.position, n.size))
            .or_else(|| {
                self.doc
                    .charts
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| (c.position, c.size))
            })?;
        let rect = Rect::from_min_size(self.screen(origin), vec2(size[0], size[1]) * self.zoom);
        resize_handles(rect)
            .into_iter()
            .find(|(p, _)| p.distance(pos) <= 8.)
            .map(|(_, handle)| (id, origin, size, handle))
    }

    pub(super) fn manipulation_cursor(&self, pos: Option<Pos2>) -> egui::CursorIcon {
        match self.drag {
            Some(Drag::Resize { handle, .. }) => return resize_cursor(handle),
            Some(Drag::Move { .. } | Drag::FreeConnector { .. } | Drag::Pan { .. }) => {
                return egui::CursorIcon::Grabbing;
            }
            Some(
                Drag::Endpoint { .. }
                | Drag::Waypoint { .. }
                | Drag::DrawPath { .. }
                | Drag::Marquee { .. }
                | Drag::Connect { .. },
            ) => return egui::CursorIcon::Crosshair,
            _ => {}
        }
        let Some(pos) = pos else {
            return egui::CursorIcon::Default;
        };
        if self.connection_handle_at(pos).is_some() {
            return egui::CursorIcon::Crosshair;
        }
        if let Some((_, _, _, handle)) = self.resize_handle_at(pos) {
            return resize_cursor(handle);
        }
        if let Some(e) = self
            .selected
            .first()
            .filter(|_| self.selected.len() == 1)
            .and_then(|id| self.doc.connectors.iter().find(|e| e.id == *id))
        {
            let route = self.doc.route(e);
            let endpoint = route
                .first()
                .into_iter()
                .chain(route.last())
                .any(|p| self.screen(*p).distance(pos) <= 9.);
            if endpoint
                || e.waypoints
                    .iter()
                    .any(|p| self.screen(*p).distance(pos) <= 9.)
                || self
                    .segment_handles(e.id)
                    .iter()
                    .any(|p| self.screen(*p).distance(pos) <= 7.)
            {
                return egui::CursorIcon::Crosshair;
            }
        }
        let world = self.world(pos);
        if self.doc.hit_node(world).is_some()
            || self.doc.hit_chart(world).is_some()
            || self.doc.hit_drawing(world, 7. / self.zoom).is_some()
        {
            return egui::CursorIcon::Grab;
        }
        if let Some(id) = self.doc.hit_connector(world, 7. / self.zoom) {
            return if self
                .doc
                .connectors
                .iter()
                .any(|e| e.id == id && (e.from == 0 || e.to == 0))
            {
                egui::CursorIcon::Grab
            } else {
                egui::CursorIcon::PointingHand
            };
        }
        egui::CursorIcon::Default
    }

    pub(super) fn paint_manipulation_hint(&self, p: &egui::Painter) {
        let description = match self.drag {
            Some(Drag::Resize { id, .. }) => {
                let size = self
                    .doc
                    .node(id)
                    .map(|n| n.size)
                    .or_else(|| self.doc.charts.iter().find(|c| c.id == id).map(|c| c.size));
                let Some(size) = size else {
                    return;
                };
                format!(
                    "{:.1} × {:.1} px · Maj : proportions · ⌥ : libre · Échap : annuler",
                    size[0], size[1]
                )
            }
            Some(Drag::Move { .. }) => {
                let rect = self
                    .placement_selection()
                    .iter()
                    .map(|(_, r)| *r)
                    .reduce(Rect::union);
                let coordinates = rect
                    .map(|r| format!("X {:.1} · Y {:.1} px", r.min.x, r.min.y))
                    .unwrap_or_else(|| "Déplacer la sélection".into());
                format!(
                    "{}{coordinates} · ⌥ : libre · Échap : annuler",
                    if self.move_guides.is_empty() {
                        ""
                    } else {
                        "Aligné · "
                    }
                )
            }
            Some(Drag::FreeConnector { .. }) => {
                "Déplacer le lien · ⌥ : libre · Échap : annuler".into()
            }
            Some(Drag::Waypoint { .. }) => "Déplacer le coude · ⌥ : libre · Échap : annuler".into(),
            Some(Drag::DrawPath { .. }) => "Dessiner le tracé · Échap : annuler".into(),
            _ => return,
        };
        let mut p = p.clone();
        p.set_clip_rect(self.canvas);
        let color = if self.move_guides.is_empty() {
            INK
        } else {
            Color32::from_rgb(196, 65, 145)
        };
        let text = p.layout(
            description,
            FontId::proportional(12.),
            color,
            (self.canvas.width() - 48.).clamp(48., 620.),
        );
        let size = text.size() + vec2(16., 12.);
        // Above the zoom bar even on a narrow canvas; wrapped and canvas-clipped.
        let origin = self.canvas.left_bottom() + vec2(16., -76. - size.y);
        let rect = Rect::from_min_size(origin, size);
        p.rect_filled(rect, 6, Color32::WHITE);
        p.rect_stroke(rect, 6, Stroke::new(1., BORDER), StrokeKind::Inside);
        p.galley(origin + vec2(8., 6.), text, color);
    }
}
