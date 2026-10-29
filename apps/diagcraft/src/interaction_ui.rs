use super::*;

pub(super) fn resize_handles(r: Rect) -> [(Pos2, [i8; 2]); 8] {
    [
        (r.left_top(), [-1, -1]),
        (r.center_top(), [0, -1]),
        (r.right_top(), [1, -1]),
        (r.right_center(), [1, 0]),
        (r.right_bottom(), [1, 1]),
        (r.center_bottom(), [0, 1]),
        (r.left_bottom(), [-1, 1]),
        (r.left_center(), [-1, 0]),
    ]
}
pub(super) fn connection_buttons(r: Rect) -> [(Pos2, Port); 4] {
    [
        (r.center_top() - vec2(0., 25.), Port::Top),
        (r.right_center() + vec2(25., 0.), Port::Right),
        (r.center_bottom() + vec2(0., 25.), Port::Bottom),
        (r.left_center() - vec2(25., 0.), Port::Left),
    ]
}
pub(super) fn paint_pattern(
    p: &egui::Painter,
    points: &[Pos2],
    stroke: Stroke,
    pattern: LinePattern,
) {
    if pattern == LinePattern::Solid {
        p.add(Shape::line(points.to_vec(), stroke));
        return;
    }
    let w = stroke.width.max(0.5);
    let period = if pattern == LinePattern::Dotted {
        4. * w
    } else {
        10. * w
    };
    let clip = p.clip_rect().expand(w);
    let mut offset = 0.;
    for segment in points.windows(2) {
        let a = segment[0];
        let delta = segment[1] - a;
        let length = delta.length();
        if length < 0.0001 {
            continue;
        }
        if let Some((start, end)) = clip_interval(a, delta, clip) {
            let lo = offset + start * length;
            let hi = offset + end * length;
            if pattern == LinePattern::Dotted {
                let mut at = (lo / period).ceil() * period;
                while at <= hi {
                    p.circle_filled(a + delta * ((at - offset) / length), w / 2., stroke.color);
                    at += period;
                }
            } else {
                let mut at = lo - lo.rem_euclid(period);
                while at <= hi {
                    let x = lo.max(at);
                    let y = hi.min(at + 6. * w);
                    if y > x {
                        p.line_segment(
                            [
                                a + delta * ((x - offset) / length),
                                a + delta * ((y - offset) / length),
                            ],
                            stroke,
                        );
                    }
                    at += period;
                }
            }
        }
        offset += length;
    }
}
pub(super) fn clip_interval(a: Pos2, d: Vec2, r: Rect) -> Option<(f32, f32)> {
    let mut lo = 0_f32;
    let mut hi = 1_f32;
    for (origin, delta, min, max) in [
        (a.x, d.x, r.left(), r.right()),
        (a.y, d.y, r.top(), r.bottom()),
    ] {
        if delta.abs() < 0.000001 {
            if origin < min || origin > max {
                return None;
            }
        } else {
            let t1 = (min - origin) / delta;
            let t2 = (max - origin) / delta;
            lo = lo.max(t1.min(t2));
            hi = hi.min(t1.max(t2));
        }
    }
    (lo <= hi).then_some((lo, hi))
}
pub(super) fn paint_head(
    p: &egui::Painter,
    points: &[Pos2],
    color: Color32,
    size: f32,
    width: f32,
    head: ArrowHead,
    background: Color32,
) {
    let coords = points.iter().map(|v| [v.x, v.y]).collect::<Vec<_>>();
    if let Some(marker) = stroke_style::marker(&coords, head, size) {
        let pts = marker
            .points
            .into_iter()
            .map(|v| pos2(v[0], v[1]))
            .collect::<Vec<_>>();
        if marker.closed {
            p.add(Shape::convex_polygon(
                pts,
                if marker.filled { color } else { background },
                if marker.filled {
                    Stroke::NONE
                } else {
                    Stroke::new(width, color)
                },
            ));
        } else {
            p.add(Shape::line(pts, Stroke::new(width, color)));
        }
    }
}
impl DrawApp {
    pub(super) fn paint_connection_buttons(&self, p: &egui::Painter) {
        if !matches!(self.tool, Tool::Select | Tool::Connect)
            || self.drag.is_some()
            || self.connect_from.is_some()
            || self.context_open
        {
            return;
        }
        let Some(n) = self.connection_handle_node() else {
            return;
        };
        let pointer = p.ctx().input(|i| i.pointer.hover_pos());
        for (at, port) in connection_buttons(self.node_screen_rect(n)) {
            let hovered = pointer.is_some_and(|pos| pos.distance(at) <= 13.);
            p.circle(
                at,
                if hovered { 11. } else { 9. },
                if hovered { ACCENT } else { Color32::WHITE },
                Stroke::new(1.8, ACCENT),
            );
            let v = match port {
                Port::Top => vec2(0., -1.),
                Port::Bottom => vec2(0., 1.),
                Port::Left => vec2(-1., 0.),
                _ => vec2(1., 0.),
            };
            let side = vec2(-v.y, v.x);
            p.line_segment(
                [at - v * 4., at + v * 4.],
                Stroke::new(1.6, if hovered { Color32::WHITE } else { ACCENT }),
            );
            p.add(Shape::line(
                vec![at + v - side * 3., at + v * 4., at + v + side * 3.],
                Stroke::new(1.6, if hovered { Color32::WHITE } else { ACCENT }),
            ));
            if hovered {
                connection_ui::paint_link_hint(
                    p,
                    self.canvas,
                    at,
                    "Glissez vers un bloc, ou cliquez pour relier",
                    ACTION_TEXT,
                );
            }
        }
    }
    #[cfg(test)]
    pub(super) fn selected_node_rect(&self) -> Option<Rect> {
        if self.selected.len() != 1 {
            return None;
        }
        self.doc.node(self.selected[0]).map(|n| {
            Rect::from_min_size(
                self.screen(n.position),
                vec2(n.size[0], n.size[1]) * self.zoom,
            )
        })
    }
    pub(super) fn segment_handles(&self, id: u64) -> Vec<Point> {
        let Some(e) = self
            .doc
            .connectors
            .iter()
            .find(|e| e.id == id && e.kind == ConnectorKind::Elbow && e.waypoints.len() < 16)
        else {
            return vec![];
        };
        self.cached_route(e.id)
            .windows(2)
            .filter(|p| (p[0][0] - p[1][0]).hypot(p[0][1] - p[1][1]) * self.zoom > 36.)
            .map(|p| [(p[0][0] + p[1][0]) / 2., (p[0][1] + p[1][1]) / 2.])
            .collect()
    }
}
