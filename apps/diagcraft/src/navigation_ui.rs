use super::*;

impl DrawApp {
    pub(super) fn canvas_navigation(&mut self, ui: &egui::Ui, over: bool, anchor: Option<Pos2>) {
        // A gesture must never turn an object drag into an accidental edit. A
        // pending connection, however, can be navigated without touching the doc.
        if !over
            || !matches!(
                self.drag,
                None | Some(Drag::Connect { .. } | Drag::Pan { .. })
            )
        {
            return;
        }
        let over_text = anchor.is_some_and(|p| {
            self.rich_editor
                .as_ref()
                .is_some_and(|e| e.content_rect.contains(p))
        });
        let events = ui.input(|i| i.events.clone());
        let options = ui.ctx().options(|o| o.input_options);
        let anchor = anchor.unwrap_or(self.canvas.center());
        let old_pan = self.pan;
        let mut navigated = false;
        for event in events {
            match event {
                egui::Event::MouseWheel {
                    unit,
                    mut delta,
                    modifiers,
                } if delta.is_finite()
                    && delta != Vec2::ZERO
                    && (!over_text || modifiers.command || modifiers.ctrl) =>
                {
                    delta = match unit {
                        egui::MouseWheelUnit::Point => delta,
                        egui::MouseWheelUnit::Line => delta * options.line_scroll_speed,
                        egui::MouseWheelUnit::Page => delta * self.canvas.size(),
                    };
                    if modifiers.command || modifiers.ctrl {
                        let exponent =
                            ((delta.x + delta.y) * options.scroll_zoom_speed).clamp(-4., 4.);
                        self.set_zoom(self.zoom * exponent.exp(), anchor);
                    } else {
                        if modifiers.shift {
                            // macOS already reports horizontal deltas for Shift.
                            delta = vec2(delta.x + delta.y, 0.);
                        }
                        // Native pixel deltas include macOS momentum and its
                        // scrolling direction. Extra smoothing adds latency and
                        // replays movement after fingers have stopped.
                        self.pan += delta;
                        self.pan = self
                            .pan
                            .clamp(vec2(-1_000_000., -1_000_000.), vec2(1_000_000., 1_000_000.));
                    }
                    navigated = true;
                }
                egui::Event::Zoom(factor) if factor.is_finite() && factor > 0. => {
                    // Read the native pinch once, separately from wheel zoom:
                    // InputState::zoom_delta also contains smoothed wheel tails.
                    self.set_zoom(self.zoom * factor, anchor);
                    navigated = true;
                }
                _ => {}
            }
        }
        if navigated {
            self.fit_next = false;
            if let Some(Drag::Pan { original, .. }) = &mut self.drag {
                *original += self.pan - old_pan;
            }
            ui.input_mut(|i| {
                i.raw_scroll_delta = Vec2::ZERO;
                i.smooth_scroll_delta = Vec2::ZERO;
            });
        }
    }
}
