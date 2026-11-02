//! Line labels stay attached to their path while their offset is freely editable.
use super::*;

pub(super) struct Click {
    id: u64,
    point: Point,
    route: Vec<Point>,
    page: usize,
    tab: usize,
}

fn label_job(label: &str, zoom: f32, color: Color32) -> egui::text::LayoutJob {
    let layout = diagcraft::text_layout::line_label(label);
    let mut job = egui::text::LayoutJob::simple_format(
        layout.text(),
        egui::TextFormat {
            font_id: FontId::proportional(14. * zoom),
            line_height: Some(layout.line_height * zoom),
            color,
            ..Default::default()
        },
    );
    job.halign = Align::Center;
    job
}

impl DrawApp {
    pub(super) fn is_labelled_line(&self, id: u64) -> bool {
        self.doc.connectors.iter().any(|e| e.id == id)
            || self.doc.drawings.iter().any(|d| d.id == id)
    }

    fn line_route(&self, id: u64) -> Option<Vec<Point>> {
        if self.doc.connectors.iter().any(|e| e.id == id) {
            Some(self.cached_route(id))
        } else {
            self.doc
                .drawings
                .iter()
                .find(|d| d.id == id)
                .map(|d| d.points())
        }
    }

    fn remember_line_click(&mut self, id: u64, point: Point) {
        self.line_label_click = self.line_route(id).map(|route| Click {
            id,
            point,
            route,
            page: self.doc.page_index,
            tab: self.active_tab,
        });
    }

    fn line_click_position(&self, id: u64) -> Option<Point> {
        let click = self.line_label_click.as_ref()?;
        (click.id == id
            && click.page == self.doc.page_index
            && click.tab == self.active_tab
            && self.line_route(id).as_ref() == Some(&click.route))
        .then_some(click.point)
    }

    pub(super) fn edit_selected_text(&mut self) {
        let Some(&id) = self.selected.first().filter(|_| self.selected.len() == 1) else {
            return;
        };
        if self.is_labelled_line(id) {
            self.open_line_label_at(id, self.line_click_position(id));
        } else {
            self.open_node_text(id);
        }
    }

    pub(super) fn begin_line_label_typing(&mut self, ctx: &egui::Context) -> bool {
        if self.tool != Tool::Select
            || egui::Popup::is_any_open(ctx)
            || self.drag.is_some()
            || self.rich_editor.is_some()
            || self.selected.len() != 1
            || !self.selection_editable()
            || !self.is_labelled_line(self.selected[0])
        {
            return false;
        }
        let typing = ctx.input(|i| {
            i.events.iter().any(|e| match e {
                egui::Event::Text(text) => {
                    !i.modifiers.command && !i.modifiers.ctrl && !text.is_empty()
                }
                egui::Event::Paste(text) => {
                    !text.is_empty() && serde_json::from_str::<Document>(text).is_err()
                }
                egui::Event::Ime(egui::ImeEvent::Preedit(text) | egui::ImeEvent::Commit(text)) => {
                    !text.is_empty()
                }
                _ => false,
            })
        });
        if typing {
            self.edit_selected_text();
        }
        typing && self.rich_editor.is_some()
    }

    #[cfg(test)]
    fn line_label_geometry(&self, id: u64) -> Option<(&str, Point)> {
        if let Some(edge) = self.doc.connectors.iter().find(|e| e.id == id) {
            Some((&edge.label, edge.label_position(&self.cached_route(id))))
        } else {
            self.doc
                .drawings
                .iter()
                .find(|d| d.id == id)
                .map(|d| (d.label.as_str(), d.label_position()))
        }
    }

    #[cfg(test)]
    pub(super) fn line_label_rect(&self, ctx: &egui::Context, id: u64) -> Option<Rect> {
        let (label, point) = self.line_label_geometry(id)?;
        if label.is_empty() {
            return None;
        }
        let galley = ctx.fonts_mut(|fonts| fonts.layout_job(label_job(label, self.zoom, INK)));
        Some(Rect::from_center_size(self.screen(point), galley.size()).expand(3. * self.zoom))
    }

    // Match the canvas paint order, including objects covering labels in the same layer.
    fn object_stack(&self, id: u64) -> (usize, usize, usize) {
        let layer = self.doc.layer_order(self.doc.object_layer(id));
        for (kind, index) in [
            self.doc.drawings.iter().position(|o| o.id == id),
            self.doc.connectors.iter().position(|o| o.id == id),
            self.doc.nodes.iter().position(|o| o.id == id),
            self.doc.charts.iter().position(|o| o.id == id),
        ]
        .into_iter()
        .enumerate()
        {
            if let Some(index) = index {
                return (layer, kind, index);
            }
        }
        (layer, 0, 0)
    }

    pub(super) fn hit_line_label(&self, ctx: &egui::Context, pos: Pos2) -> Option<u64> {
        let view = self.doc.organization_view();
        let covered_by = self
            .organized_hit(self.world(pos))
            .map(|id| self.object_stack(id));
        let routes = self.route_cache.borrow_mut().scene(&self.doc);
        self.doc
            .drawings
            .iter()
            .enumerate()
            .filter(|(_, d)| !d.label.is_empty())
            .map(|(i, d)| {
                (
                    d.id,
                    (view.layer_order(view.object_layer(d.id)), 0, i),
                    d.label.as_str(),
                    d.label_position(),
                )
            })
            .chain(
                self.doc
                    .connectors
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| !e.label.is_empty())
                    .map(|(i, e)| {
                        (
                            e.id,
                            (view.layer_order(view.object_layer(e.id)), 1, i),
                            e.label.as_str(),
                            e.label_position(&routes[&e.id].points),
                        )
                    }),
            )
            .filter(|(id, _, _, _)| view.is_object_visible(*id))
            .filter(|(_, order, _, _)| covered_by.is_none_or(|above| above <= *order))
            .filter(|(_, _, label, point)| {
                let center = self.screen(*point);
                // A conservative bound avoids laying out distant labels during hit testing.
                let width =
                    label.split('\n').map(str::len).max().unwrap_or(0) as f32 * 28. * self.zoom
                        + 20.;
                let height = label.split('\n').count() as f32 * 28. * self.zoom + 20.;
                if !Rect::from_center_size(center, vec2(width, height)).contains(pos) {
                    return false;
                }
                let galley =
                    ctx.fonts_mut(|fonts| fonts.layout_job(label_job(label, self.zoom, INK)));
                Rect::from_center_size(center, galley.size())
                    .expand(3. * self.zoom + 5.)
                    .contains(pos)
            })
            .max_by_key(|(_, order, _, _)| *order)
            .map(|(id, _, _, _)| id)
    }

    pub(super) fn paint_line_label(
        &self,
        p: &egui::Painter,
        id: u64,
        label: &str,
        point: Point,
        color: Color32,
    ) {
        if self.editing_line_label(id) {
            return;
        }
        if label.is_empty() {
            return;
        }
        let galley = p.layout_job(label_job(label, self.zoom, color));
        let rect = Rect::from_center_size(self.screen(point), galley.size());
        p.rect_filled(rect.expand(3. * self.zoom), 2, Color32::WHITE);
        p.galley(rect.min - galley.rect.min.to_vec2(), galley, color);
        if self.selected.contains(&id) && self.selection_editable() {
            let center = rect.right_bottom() + vec2(3. * self.zoom, 3. * self.zoom);
            p.add(Shape::convex_polygon(
                vec![
                    center - vec2(0., 4.),
                    center + vec2(4., 0.),
                    center + vec2(0., 4.),
                    center - vec2(4., 0.),
                ],
                Color32::WHITE,
                Stroke::new(1., ACCENT),
            ));
        }
    }

    pub(super) fn start_line_label_drag(
        &mut self,
        ctx: &egui::Context,
        pos: Pos2,
        additive: bool,
    ) -> bool {
        let Some(id) = self.hit_line_label(ctx, pos) else {
            return false;
        };
        self.select_for_drag(id, self.world(pos), additive);
        if !additive
            && self.selected.contains(&id)
            && self.selection_editable()
            && self.doc.can_edit_object(id)
        {
            let offset = self
                .doc
                .connectors
                .iter()
                .find(|e| e.id == id)
                .map(|e| e.label_offset)
                .or_else(|| {
                    self.doc
                        .drawings
                        .iter()
                        .find(|d| d.id == id)
                        .map(|d| d.label_offset)
                })
                .unwrap();
            self.drag = Some(Drag::LineLabel {
                id,
                start: self.world(pos),
                offset,
            });
        } else {
            self.drag = None;
            self.drag_snapshot = None;
        }
        true
    }

    pub(super) fn line_label_clicked(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        pos: Option<Pos2>,
    ) {
        if !response.clicked() || self.tool != Tool::Select || ui.input(|i| i.modifiers.any()) {
            return;
        }
        let Some(pos) = pos else {
            return;
        };
        let id = self
            .hit_line_label(ui.ctx(), pos)
            .or_else(|| self.organized_hit(self.world(pos)));
        if let Some(id) = id.filter(|id| {
            self.selected.contains(id)
                && self.is_labelled_line(*id)
                && self.doc.can_edit_object(*id)
        }) {
            let double_clicked = response.double_clicked()
                && self.line_label_click.as_ref().is_some_and(|previous| {
                    previous.id == id
                        && previous.page == self.doc.page_index
                        && previous.tab == self.active_tab
                        && self.screen(previous.point).distance(pos) <= 6.
                });
            self.remember_line_click(id, self.world(pos));
            if double_clicked {
                self.open_line_label_at(id, Some(self.world(pos)));
            }
        } else {
            self.line_label_click = None;
        }
    }
}
