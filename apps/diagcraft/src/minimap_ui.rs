//! An inexpensive, camera-only overview of the active page.
use super::*;

pub(super) const MAP_ID: &str = "minimap_navigation";

#[derive(Default)]
pub(super) struct State {
    drag: Option<NavigationDrag>,
    frame_rect: Option<Rect>,
    block_until_release: bool,
    blocks_this_frame: bool,
    opener: Option<egui::Id>,
    restore_opener: bool,
    captured_buttons: [bool; egui::NUM_POINTER_BUTTONS],
}

impl State {
    pub(super) fn remember_opener(&mut self, response: &egui::Response) {
        self.opener = Some(response.id);
        if self.restore_opener {
            response.request_focus();
            self.restore_opener = false;
        }
    }

    pub(super) fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Transform {
    pub(super) world: Rect,
    pub(super) map: Rect,
}

impl Transform {
    fn new(world: Rect, body: Rect) -> Self {
        let scale = (body.width() / world.width()).min(body.height() / world.height());
        Self {
            world,
            map: Rect::from_center_size(body.center(), world.size() * scale),
        }
    }

    pub(super) fn project(self, point: Point) -> Pos2 {
        self.map.min
            + (pos2(point[0], point[1]) - self.world.min) * (self.map.width() / self.world.width())
    }

    pub(super) fn unproject(self, point: Pos2) -> Point {
        let p = self.world.min + (point - self.map.min) * (self.world.width() / self.map.width());
        [p.x, p.y]
    }

    fn project_rect(self, rect: Rect) -> Rect {
        Rect::from_min_max(
            self.project([rect.left(), rect.top()]),
            self.project([rect.right(), rect.bottom()]),
        )
    }
}

#[derive(Clone, Copy)]
struct NavigationDrag {
    transform: Transform,
    panel: Rect,
    tab: usize,
    page: usize,
    offset: Vec2,
    original_pan: Vec2,
}

struct BoxItem {
    id: u64,
    rect: Rect,
    kind: ShapeKind,
}
struct LineItem {
    id: u64,
    points: Vec<Point>,
}
struct Scene {
    world: Rect,
    boxes: Vec<BoxItem>,
    lines: Vec<LineItem>,
}

impl Scene {
    fn capture(app: &DrawApp) -> Self {
        let view = app.doc.organization_view();
        let mut scene = Self {
            world: Rect::from_min_size(
                Pos2::ZERO,
                vec2(app.doc.page_size[0], app.doc.page_size[1]),
            ),
            boxes: vec![],
            lines: vec![],
        };
        for n in &app.doc.nodes {
            if view.is_object_visible(n.id) {
                scene.add_box(n.id, n.position, n.size, n.kind);
            }
        }
        for c in &app.doc.charts {
            if view.is_object_visible(c.id) {
                scene.add_box(c.id, c.position, c.size, ShapeKind::Rectangle);
            }
        }
        for d in &app.doc.drawings {
            if view.is_object_visible(d.id) {
                scene.add_line(d.id, d.points());
            }
        }
        // Reuse the exact canvas routes once; never export/rasterize a diagram
        // or rebuild the cache separately for each connector in this overview.
        if !app.doc.connectors.is_empty() {
            let routes = app.route_cache.borrow_mut().scene(&app.doc);
            for e in &app.doc.connectors {
                if view.is_object_visible(e.id)
                    && let Some(route) = routes.get(&e.id)
                {
                    scene.add_line(e.id, route.points.clone());
                }
            }
        }
        scene.world = scene.world.expand2(scene.world.size() * 0.03);
        scene
    }

    fn add_box(&mut self, id: u64, position: Point, size: Point, kind: ShapeKind) {
        let rect = Rect::from_min_size(pos2(position[0], position[1]), vec2(size[0], size[1]));
        self.world = self.world.union(rect);
        self.boxes.push(BoxItem { id, rect, kind });
    }

    fn add_line(&mut self, id: u64, points: Vec<Point>) {
        for point in &points {
            self.world.extend_with(pos2(point[0], point[1]));
        }
        self.lines.push(LineItem { id, points });
    }
}

pub(super) fn toggle_button(ui: &mut egui::Ui, open: bool) -> egui::Response {
    let response = ui.add(
        egui::Button::new("")
            .min_size(vec2(30., 30.))
            .selected(open),
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            ui.is_enabled(),
            t(if open {
                "Masquer la mini-carte"
            } else {
                "Afficher la mini-carte"
            }),
        )
    });
    let ink = if open { ACTION_TEXT } else { INK };
    let page = Rect::from_center_size(response.rect.center(), vec2(18., 14.));
    ui.painter()
        .rect_stroke(page, 2, Stroke::new(1.4, ink), StrokeKind::Inside);
    ui.painter().rect_stroke(
        Rect::from_min_size(page.min + vec2(5., 4.), vec2(10., 7.)),
        1,
        Stroke::new(1.4, ink),
        StrokeKind::Inside,
    );
    focus_ui::outline(ui, &response);
    response.on_hover_text(t("Afficher / masquer la mini-carte"))
}

impl DrawApp {
    /// Geometry is known before canvas input, including on the opening frame.
    pub(super) fn panel_rect(&self) -> Option<Rect> {
        if !self.show_minimap
            || !self.canvas.is_finite()
            || self.drag.is_some()
            || self.keyboard_move.is_some()
            || self.rich_editor.is_some()
            || self.context_open
        {
            return None;
        }
        let size = vec2(
            (self.canvas.width() - 32.).min(200.),
            (self.canvas.height() - 156.).min(170.),
        );
        if size.x < 140. || size.y < 90. {
            return None;
        }
        Some(Rect::from_min_size(
            self.canvas.left_bottom() + vec2(16., -76. - size.y),
            size,
        ))
    }

    #[cfg(test)]
    pub(super) fn minimap_transform(&self) -> Option<Transform> {
        let rect = self.panel_rect()?;
        Some(Transform::new(Scene::capture(self).world, map_body(rect)))
    }

    pub(super) fn minimap_blocks_canvas(&self, ctx: &egui::Context) -> bool {
        self.minimap_ui.blocks_this_frame
            || self.minimap_ui.is_dragging()
            || ctx.input(|i| i.pointer.hover_pos()).is_some_and(|point| {
                self.minimap_ui
                    .frame_rect
                    .is_some_and(|rect| rect.contains(point))
            })
    }

    pub(super) fn minimap(&mut self, ctx: &egui::Context, interactive: bool) {
        let down = ctx.input(|i| i.pointer.primary_down());
        self.minimap_ui.blocks_this_frame = self.minimap_ui.is_dragging()
            || self.minimap_ui.block_until_release
            || self
                .minimap_ui
                .captured_buttons
                .iter()
                .any(|captured| *captured);
        // Capture the complete transaction, including a release outside the
        // overview on its opening frame before egui has a prior hit-test layer.
        for button in [
            egui::PointerButton::Primary,
            egui::PointerButton::Secondary,
            egui::PointerButton::Middle,
            egui::PointerButton::Extra1,
            egui::PointerButton::Extra2,
        ] {
            if !ctx.input(|i| i.pointer.button_down(button)) {
                self.minimap_ui.captured_buttons[button as usize] = false;
            }
        }
        if !down {
            self.minimap_ui.block_until_release = false;
        }
        let panel = self.panel_rect().filter(|_| interactive);
        self.minimap_ui.frame_rect = panel;
        let Some(panel) = panel else {
            if self.minimap_ui.drag.take().is_some() {
                self.minimap_ui.block_until_release = down;
            }
            return;
        };
        ctx.input(|i| {
            for event in &i.events {
                if let egui::Event::PointerButton {
                    pos,
                    button,
                    pressed: true,
                    ..
                } = event
                    && panel.contains(*pos)
                {
                    self.minimap_ui.captured_buttons[*button as usize] = true;
                    self.minimap_ui.blocks_this_frame = true;
                }
            }
        });
        let scene = Scene::capture(self);
        let transform = Transform::new(scene.world, map_body(panel));
        if self.minimap_ui.drag.is_some_and(|drag| {
            drag.transform != transform
                || drag.panel != panel
                || drag.tab != self.active_tab
                || drag.page != self.doc.page_index
                || !ctx.input(|i| i.focused)
                || !ctx.memory(|m| m.has_focus(egui::Id::new(MAP_ID)))
        }) {
            self.minimap_ui.drag = None;
            self.minimap_ui.block_until_release = down;
        }
        let mut close = false;
        egui::Area::new("minimap_panel".into())
            .order(egui::Order::Foreground)
            .fixed_pos(panel.min)
            .movable(false)
            .show(ctx, |ui| {
                ui.set_min_size(panel.size());
                ui.painter().rect(panel, 7, Color32::WHITE, Stroke::new(1., BORDER), StrokeKind::Inside);
                let title = Rect::from_min_max(panel.min + vec2(10., 3.), pos2(panel.right() - 38., panel.top() + 27.));
                ui.put(title, egui::Label::new(RichText::new(t("Mini-carte")).size(12.).color(INK)).truncate());
                let button = ui.put(
                    Rect::from_min_size(panel.right_top() + vec2(-32., 1.), vec2(30., 30.)),
                    egui::Button::new("×").frame(false),
                );
                button.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, t("Masquer la mini-carte")));
                focus_ui::outline(ui, &button);
                close = button.on_hover_text(t("Masquer la mini-carte")).clicked();
                let response = ui.interact(map_body(panel), egui::Id::new(MAP_ID), Sense::click_and_drag());
                response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, t("Naviguer dans la mini-carte")));
                let response = response.on_hover_text(t("Cliquez ou glissez pour déplacer la vue. Flèches : déplacer · Début : ajuster la page."));
                if !self.minimap_ui.block_until_release {
                    if response.drag_started_by(egui::PointerButton::Primary)
                        && let Some(press) = ctx.input(|i| i.pointer.press_origin())
                    {
                        response.request_focus();
                        let point = transform.unproject(press);
                        let center = self.world(self.canvas.center());
                        let offset = if transform.project_rect(self.world_viewport()).contains(press) {
                            pos2(point[0], point[1]) - pos2(center[0], center[1])
                        } else {
                            Vec2::ZERO
                        };
                        self.minimap_ui.drag = Some(NavigationDrag {
                            transform, panel, tab: self.active_tab, page: self.doc.page_index,
                            offset, original_pan: self.pan,
                        });
                    }
                    if let Some(drag) = self.minimap_ui.drag {
                        if let Some(pointer) = ctx.input(|i| i.pointer.interact_pos()) {
                            let world = drag.transform.unproject(pointer);
                            self.center_minimap([world[0] - drag.offset.x, world[1] - drag.offset.y]);
                        }
                        if !down {
                            self.minimap_ui.drag = None;
                        }
                    } else if response.clicked()
                        && let Some(pointer) = response.interact_pointer_pos()
                    {
                        response.request_focus();
                        self.center_minimap(transform.unproject(transform.map.clamp(pointer)));
                    }
                }
                if response.has_focus() {
                    ctx.memory_mut(|m| m.set_focus_lock_filter(response.id, egui::EventFilter {
                        horizontal_arrows: true, vertical_arrows: true, escape: true, ..Default::default()
                    }));
                    self.minimap_keyboard(ctx);
                }
                paint(ui.painter(), &scene, transform, self.world_viewport(), &self.selected, self.doc.page_size);
                focus_ui::outline(ui, &response);
                if response.hovered() || self.minimap_ui.is_dragging() {
                    ctx.set_cursor_icon(if down { egui::CursorIcon::Grabbing } else { egui::CursorIcon::Grab });
                }
            });
        if close {
            self.show_minimap = false;
            self.minimap_ui.drag = None;
            self.minimap_ui.blocks_this_frame = true;
            self.minimap_ui.restore_opener = true;
            if let Some(id) = self.minimap_ui.opener {
                ctx.memory_mut(|m| m.request_focus(id));
            }
        }
    }

    fn world_viewport(&self) -> Rect {
        let a = self.world(self.canvas.min);
        let b = self.world(self.canvas.max);
        Rect::from_min_max(pos2(a[0], a[1]), pos2(b[0], b[1]))
    }

    fn center_minimap(&mut self, point: Point) {
        if point.iter().all(|v| v.is_finite()) {
            self.pan = self.canvas.size() / 2. - vec2(point[0], point[1]) * self.zoom;
            self.fit_next = false;
        }
    }

    fn minimap_keyboard(&mut self, ctx: &egui::Context) {
        let mut delta = Vec2::ZERO;
        for (key, direction) in [
            (Key::ArrowLeft, vec2(-1., 0.)),
            (Key::ArrowRight, vec2(1., 0.)),
            (Key::ArrowUp, vec2(0., -1.)),
            (Key::ArrowDown, vec2(0., 1.)),
        ] {
            let scale = if ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, key)) {
                0.25
            } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, key)) {
                0.1
            } else {
                0.
            };
            delta += direction * scale;
        }
        if delta != Vec2::ZERO {
            self.pan -= delta * self.canvas.size();
            self.fit_next = false;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Home)) {
            self.minimap_ui.drag = None;
            self.fit();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape)) {
            if let Some(drag) = self.minimap_ui.drag.take() {
                self.pan = drag.original_pan;
                self.minimap_ui.block_until_release = ctx.input(|i| i.pointer.primary_down());
            }
            ctx.memory_mut(|m| m.surrender_focus(egui::Id::new(MAP_ID)));
        }
    }
}

fn map_body(panel: Rect) -> Rect {
    Rect::from_min_max(panel.min + vec2(10., 32.), panel.max - vec2(10., 10.))
}

fn paint(
    painter: &egui::Painter,
    scene: &Scene,
    transform: Transform,
    viewport: Rect,
    selected: &[u64],
    page_size: Point,
) {
    let selected: std::collections::HashSet<_> = selected.iter().copied().collect();
    let p = painter.with_clip_rect(transform.map.expand(2.));
    p.rect_filled(transform.map, 2, theme_ui::HELP_BACKGROUND);
    let page = transform.project_rect(Rect::from_min_size(
        Pos2::ZERO,
        vec2(page_size[0], page_size[1]),
    ));
    p.rect(
        page,
        0,
        Color32::WHITE,
        Stroke::new(1., INFO_TEXT),
        StrokeKind::Inside,
    );
    for item in &scene.boxes {
        let rect = transform.project_rect(item.rect);
        let active = selected.contains(&item.id);
        let stroke = Stroke::new(
            if active { 1.5 } else { 0.8 },
            if active { ACTION_TEXT } else { INFO_TEXT },
        );
        let fill = if active {
            Color32::from_rgb(224, 238, 255)
        } else {
            Color32::from_rgb(229, 237, 244)
        };
        match item.kind {
            ShapeKind::Diamond => {
                p.add(Shape::convex_polygon(
                    vec![
                        rect.center_top(),
                        rect.right_center(),
                        rect.center_bottom(),
                        rect.left_center(),
                    ],
                    fill,
                    stroke,
                ));
            }
            ShapeKind::Ellipse => {
                p.add(egui::epaint::EllipseShape {
                    center: rect.center(),
                    radius: rect.size() / 2.,
                    fill,
                    stroke,
                });
            }
            _ => {
                p.rect(rect, 1, fill, stroke, StrokeKind::Inside);
            }
        }
    }
    for line in &scene.lines {
        let active = selected.contains(&line.id);
        p.add(Shape::line(
            line.points
                .iter()
                .map(|point| transform.project(*point))
                .collect(),
            Stroke::new(
                if active { 1.5 } else { 0.8 },
                if active { ACTION_TEXT } else { INFO_TEXT },
            ),
        ));
    }
    let frame = transform.project_rect(viewport).intersect(transform.map);
    if frame.is_positive() {
        p.rect(
            frame,
            0,
            Color32::from_rgba_unmultiplied(0, 83, 180, 18),
            Stroke::new(2., ACTION_TEXT),
            StrokeKind::Inside,
        );
    } else {
        // The drawing projection stays fixed when the view moves off the sheet.
        let inset = Rect::from_center_size(
            transform.map.center(),
            (transform.map.size() - Vec2::splat(6.)).max(Vec2::ZERO),
        );
        p.circle_filled(
            inset.clamp(transform.project([viewport.center().x, viewport.center().y])),
            3.,
            ACTION_TEXT,
        );
    }
}
