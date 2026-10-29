//! Workspace traversal is UI state, independent of the document and its undo stack.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Region {
    Ribbon,
    Library,
    Canvas,
    Inspector,
    Pages,
}

impl Region {
    const ALL: [Self; 5] = [
        Self::Ribbon,
        Self::Library,
        Self::Canvas,
        Self::Inspector,
        Self::Pages,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Ribbon => "Ruban",
            Self::Library => "Bibliothèque",
            Self::Canvas => "Feuille de dessin",
            Self::Inspector => "Propriétés",
            Self::Pages => "Pages",
        }
    }
}

#[derive(Default)]
pub(super) struct State {
    anchors: [Option<egui::Id>; 5],
    requested: Option<Region>,
}

impl State {
    pub(super) fn anchor(&mut self, ui: &egui::Ui, region: Region, r: &egui::Response) {
        self.anchors[region as usize] = Some(r.id);
        if self.requested == Some(region) {
            r.request_focus();
            r.scroll_to_me(None);
            self.requested = None;
        }
        if region != Region::Canvas {
            outline(ui, r);
        }
    }

    pub(super) fn canvas_focused(&self, ctx: &egui::Context) -> bool {
        self.anchors[Region::Canvas as usize].is_some_and(|id| ctx.memory(|m| m.has_focus(id)))
    }
}

pub(super) fn outline(ui: &egui::Ui, response: &egui::Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.shrink(2.),
            4,
            Stroke::new(2., Color32::from_rgb(0, 83, 180)),
            StrokeKind::Inside,
        );
    }
}

impl DrawApp {
    pub(super) fn ui_has_keyboard_focus(&self, ctx: &egui::Context) -> bool {
        ctx.wants_keyboard_input() && !self.focus_ui.canvas_focused(ctx)
    }

    pub(super) fn cycle_workspace_focus(&mut self, ctx: &egui::Context, backwards: bool) {
        // A field or a button reached using Tab belongs to its surrounding zone,
        // rather than to the last F6 anchor. Hidden panes keep their rail anchor.
        let focused = ctx.memory(|m| m.focused());
        let region = focused
            .and_then(|id| {
                Region::ALL
                    .into_iter()
                    .find(|region| self.focus_ui.anchors[*region as usize] == Some(id))
                    .or_else(|| {
                        ctx.read_response(id).map(|r| {
                            let p = r.rect.center();
                            if p.y < self.canvas.top() {
                                Region::Ribbon
                            } else if p.y > self.canvas.bottom() {
                                Region::Pages
                            } else if p.x < self.canvas.left() {
                                Region::Library
                            } else if p.x > self.canvas.right() {
                                Region::Inspector
                            } else {
                                Region::Canvas
                            }
                        })
                    })
            })
            .unwrap_or(Region::Canvas);
        let next = Region::ALL[(region as usize + if backwards { 4 } else { 1 }) % 5];
        self.commit();
        // Applied when the real widget is rendered, including after a command
        // palette closes. No pane is opened, moved or resized as a side effect.
        self.focus_ui.requested = Some(next);
        self.status = i18n::render(
            "Focus : {0} · F6 / Maj+F6 pour changer de zone",
            &[("0", t(next.label()))],
        );
        ctx.request_repaint();
    }
}
