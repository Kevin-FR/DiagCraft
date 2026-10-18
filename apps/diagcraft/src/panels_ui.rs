//! Compact panel controls remain available even when the wide panes are hidden.
use super::*;
impl DrawApp {
    pub(super) fn side_panels(&mut self, ctx: &egui::Context) {
        // Keep the navigation rail: collapsing the library must never hide its opener.
        self.sidebar(ctx);
        if self.show_inspector {
            self.inspector(ctx);
        } else {
            egui::SidePanel::right("inspector_rail")
                .exact_width(RAIL_WIDTH)
                .resizable(false)
                .frame(frame(true, false))
                .show(ctx, |ui| {
                    if panel_toggle(ui, false, true, "Afficher les propriétés").clicked() {
                        self.show_inspector = true;
                    }
                });
        }
    }
}

pub(super) const MIN_CANVAS: f32 = 520.;
pub(super) const RAIL_WIDTH: f32 = 48.;

pub(super) fn frame(right: bool, wide: bool) -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .inner_margin(egui::Margin {
            left: if wide {
                12
            } else if right {
                8
            } else {
                6
            },
            right: if right {
                8
            } else if wide {
                12
            } else {
                6
            },
            top: 6,
            bottom: 6,
        })
}

/// Restore the user's preferred width before applying temporary window limits.
/// egui's panel memory otherwise retains the width of a shrunken window.
pub(super) fn prepare_width(ctx: &egui::Context, name: &str, preferred: f32, min: f32, max: f32) {
    let id = egui::Id::new(name);
    let mut state = egui::containers::panel::PanelState::load(ctx, id)
        .unwrap_or(egui::containers::panel::PanelState { rect: Rect::ZERO });
    state.rect.max.x = state.rect.min.x + preferred.clamp(min, max.max(min));
    ctx.data_mut(|data| data.insert_persisted(id, state));
}

pub(super) fn remember_width(ctx: &egui::Context, name: &str, rect: Rect, preferred: &mut f32) {
    // egui 0.33's resize response distinguishes user drags from responsive clamps.
    if ctx
        .read_response(egui::Id::new(name).with("__resize"))
        .is_some_and(|r| r.dragged())
    {
        *preferred = rect.width().clamp(240., 400.);
    }
}

pub(super) fn heading(ui: &mut egui::Ui, title: &str, open: Option<&mut bool>) {
    ui.horizontal(|ui| {
        ui.set_min_height(30.);
        ui.add(egui::Label::new(RichText::new(t(title)).size(15.).strong()).truncate());
        if let Some(open) = open {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if panel_toggle(ui, true, true, "Masquer les propriétés").clicked() {
                    *open = false;
                }
            });
        }
    });
    ui.separator();
}
pub(super) fn ribbon_toggle(ui: &mut egui::Ui, expanded: bool) -> egui::Response {
    let label = if expanded {
        "Réduire le ruban"
    } else {
        "Développer le ruban"
    };
    let r = ui
        .add_sized([32., 30.], egui::Button::new(""))
        .on_hover_text(t(label));
    r.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
    let c = r.rect.center();
    ui.painter().line_segment(
        [
            c + vec2(-6., if expanded { 2. } else { -2. }),
            c + vec2(0., if expanded { -3. } else { 3. }),
        ],
        Stroke::new(1.8, INK),
    );
    ui.painter().line_segment(
        [
            c + vec2(0., if expanded { -3. } else { 3. }),
            c + vec2(6., if expanded { 2. } else { -2. }),
        ],
        Stroke::new(1.8, INK),
    );
    r
}
pub(super) fn panel_toggle(
    ui: &mut egui::Ui,
    open: bool,
    right: bool,
    label: &str,
) -> egui::Response {
    let response = ui
        .add_sized([32., 30.], egui::Button::new("").selected(open))
        .on_hover_text(t(label));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), t(label))
    });
    let r = Rect::from_center_size(response.rect.center(), vec2(20., 16.));
    let color = if open { ACCENT } else { INK };
    let stroke = Stroke::new(1.5, color);
    ui.painter().rect_stroke(r, 2, stroke, StrokeKind::Inside);
    let x = if right { r.right() - 6. } else { r.left() + 6. };
    ui.painter()
        .vline(x, r.top() + 1.5..=r.bottom() - 1.5, stroke);
    let fill = if right {
        Rect::from_min_max(pos2(x + 1., r.top() + 2.), r.right_bottom() - vec2(2., 2.))
    } else {
        Rect::from_min_max(r.left_top() + vec2(2., 2.), pos2(x - 1., r.bottom() - 2.))
    };
    ui.painter()
        .rect_filled(fill, 0, color.gamma_multiply(if open { 0.6 } else { 0.25 }));
    response
}
