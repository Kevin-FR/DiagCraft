//! Compact panel controls remain available even when the wide panes are hidden.
use super::*;
impl DrawApp {
    pub(super) fn side_panels(&mut self, ctx: &egui::Context) {
        // Keep the navigation rail: collapsing the library must never hide its opener.
        self.sidebar(ctx);
        if self.show_inspector {
            self.inspector(ctx);
        }
    }
}
pub(super) fn ribbon_toggle(ui: &mut egui::Ui, expanded: bool) -> egui::Response {
    let label = if expanded {
        "Réduire le ruban"
    } else {
        "Développer le ruban"
    };
    let r = ui
        .add_sized([32., 30.], egui::Button::new(""))
        .on_hover_text(label);
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
        .on_hover_text(label);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
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
