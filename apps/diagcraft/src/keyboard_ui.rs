//! A held arrow key is one cancellable translation, independent of document format.
use super::*;
use std::collections::HashSet;

const ARROWS: [Key; 4] = [
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::ArrowUp,
    Key::ArrowDown,
];

fn released_between_presses(input: &egui::InputState) -> bool {
    // Native input can deliver a release and the next press in one frame. Rebuild
    // the initial held set, then find the release that ended the previous gesture.
    let mut held: HashSet<_> = ARROWS
        .iter()
        .copied()
        .filter(|key| input.key_down(*key))
        .collect();
    for event in input.events.iter().rev() {
        if let egui::Event::Key {
            key,
            pressed,
            repeat,
            ..
        } = event
            && ARROWS.contains(key)
        {
            if !pressed {
                held.insert(*key);
            } else if !repeat {
                held.remove(key);
            }
        }
    }
    for event in &input.events {
        if let egui::Event::Key { key, pressed, .. } = event
            && ARROWS.contains(key)
        {
            if *pressed {
                held.insert(*key);
            } else {
                held.remove(key);
                if held.is_empty() {
                    return true;
                }
            }
        }
    }
    false
}

pub(super) struct Move {
    before: Document,
    selected: Vec<u64>,
    delta: Point,
}

impl Move {
    pub(super) fn changed(&self, doc: &Document) -> bool {
        self.before != *doc
    }
}

impl DrawApp {
    pub(super) fn keyboard_move_shortcuts(&mut self, ctx: &egui::Context) -> bool {
        // Documentation captures keep their synthetic gesture visible.
        if self.screenshot.is_some() && self.capture_alignment {
            return false;
        }
        let held = ctx.input(|i| ARROWS.iter().any(|key| i.key_down(*key)));
        if !held {
            self.keyboard_move_cancelled = false;
        }
        let Some(movement) = &self.keyboard_move else {
            return false;
        };
        // Flush before handing input to a dialog, a field or another selection.
        if self.command_dialog_blocked()
            || self.show_help
            || self.command_palette.open
            || self.context_open
            || self.ui_has_keyboard_focus(ctx)
            || movement.selected != self.selected
            || ctx.input(|i| !i.focused)
        {
            self.commit();
            self.keyboard_move_cancelled = held;
            return false;
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Escape))
            || ctx.input_mut(|i| {
                i.consume_shortcut(&egui::KeyboardShortcut::new(
                    egui::Modifiers::COMMAND,
                    Key::Z,
                ))
            })
        {
            let movement = self.keyboard_move.take().unwrap();
            self.doc = movement.before;
            self.selected = movement.selected;
            self.keyboard_move_cancelled = held;
            self.status = "Déplacement annulé · la sélection est conservée".into();
            return true;
        }
        let interrupted = ctx.input(|i| {
            i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt || i.events.iter().any(|event| {
                matches!(event, egui::Event::PointerButton { pressed: true, .. } | egui::Event::MouseWheel { .. } | egui::Event::Zoom(_) | egui::Event::Paste(_))
                    || matches!(event, egui::Event::Key { key, pressed: true, .. } if !ARROWS.contains(key))
            })
        });
        if !held || interrupted || ctx.input(released_between_presses) {
            self.commit();
            self.keyboard_move_cancelled = held && interrupted;
        }
        false
    }

    pub(super) fn selection_has_movable(&self) -> bool {
        let selected: HashSet<_> = self.selected.iter().copied().collect();
        self.doc.nodes.iter().any(|n| selected.contains(&n.id))
            || self.doc.charts.iter().any(|c| selected.contains(&c.id))
            || self.doc.drawings.iter().any(|d| selected.contains(&d.id))
            || self
                .doc
                .connectors
                .iter()
                .any(|e| selected.contains(&e.id) && (e.from == 0 || e.to == 0))
    }

    pub(super) fn nudge_selection(&mut self, raw: Point, continuous: bool) {
        self.normalize_selection();
        if !self.selection_editable() {
            self.status = "La sélection est verrouillée".into();
            return;
        }
        if self.keyboard_move_cancelled {
            return;
        }
        if self.keyboard_move.is_none() {
            self.commit();
        }
        let selected: HashSet<_> = self.selected.iter().copied().collect();
        // Intersect one range for the entire selection: paths cannot stretch and
        // blocks cannot compress against a page edge. Imported off-page objects
        // may move back towards the page without jumping there on another axis.
        let mut low = [-100_000_f32; 2];
        let mut high = [100_000_f32; 2];
        let mut include = |position: Point, size: Option<Point>| {
            for a in 0..2 {
                let (min, max) = size.map_or((-100_000., 100_000.), |size| {
                    (0., (self.doc.edit_bounds()[a] - size[a]).max(0.))
                });
                low[a] = low[a].max((min - position[a]).min(0.));
                high[a] = high[a].min((max - position[a]).max(0.));
            }
        };
        for n in self.doc.nodes.iter().filter(|n| selected.contains(&n.id)) {
            include(n.position, Some(n.size));
        }
        for c in self.doc.charts.iter().filter(|c| selected.contains(&c.id)) {
            include(c.position, Some(c.size));
        }
        for d in self
            .doc
            .drawings
            .iter()
            .filter(|d| selected.contains(&d.id))
        {
            include(d.start, None);
            include(d.end, None);
        }
        for e in self.doc.connectors.iter().filter(|e| {
            selected.contains(&e.id)
                && (e.from == 0
                    || e.to == 0
                    || (selected.contains(&e.from) && selected.contains(&e.to)))
        }) {
            for p in e
                .free_start
                .iter()
                .chain(e.free_end.iter())
                .chain(e.waypoints.iter())
            {
                include(*p, None);
            }
        }
        let delta = [0, 1].map(|a| raw[a].clamp(low[a], high[a]));
        if delta == [0., 0.] {
            self.status = "Limite de déplacement atteinte".into();
            return;
        }
        if self.keyboard_move.is_none() {
            self.keyboard_move = Some(Move {
                before: self.doc.clone(),
                selected: self.selected.clone(),
                delta: [0., 0.],
            });
        }
        let translate = |p: &mut Point| {
            for a in 0..2 {
                p[a] += delta[a];
            }
        };
        for n in self
            .doc
            .nodes
            .iter_mut()
            .filter(|n| selected.contains(&n.id))
        {
            translate(&mut n.position);
        }
        for c in self
            .doc
            .charts
            .iter_mut()
            .filter(|c| selected.contains(&c.id))
        {
            translate(&mut c.position);
        }
        for d in self
            .doc
            .drawings
            .iter_mut()
            .filter(|d| selected.contains(&d.id))
        {
            translate(&mut d.start);
            translate(&mut d.end);
        }
        for e in self.doc.connectors.iter_mut().filter(|e| {
            selected.contains(&e.id)
                && (e.from == 0
                    || e.to == 0
                    || (selected.contains(&e.from) && selected.contains(&e.to)))
        }) {
            for p in e
                .free_start
                .iter_mut()
                .chain(e.free_end.iter_mut())
                .chain(e.waypoints.iter_mut())
            {
                translate(p);
            }
        }
        let movement = self.keyboard_move.as_mut().unwrap();
        for (total, step) in movement.delta.iter_mut().zip(delta) {
            *total += step;
        }
        self.status = "Flèches : 1 px · Maj : 10 px · Échap : annuler le déplacement".into();
        // Palette/button invocation is a discrete edit, unlike a held arrow key.
        if !continuous {
            self.commit();
        }
    }

    pub(super) fn prepare_keyboard_capture(&mut self) {
        self.prepare_alignment_capture(false);
        self.show_inspector = false;
        self.doc.title = "Placer avec précision".into();
        self.nudge_selection([20., 10.], true);
        self.capture_alignment = true;
    }

    pub(super) fn keyboard_move_hint(&self) -> Option<String> {
        self.keyboard_move.as_ref().map(|m| {
            i18n::render(
                "Clavier · ΔX {0} · ΔY {1} px · Échap : annuler",
                &[("0", m.delta[0].to_string()), ("1", m.delta[1].to_string())],
            )
        })
    }
}
