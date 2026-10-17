use crate::document::Document;
use std::sync::Arc;

/// Bounded document snapshots. A pointer drag becomes one undo step.
#[derive(Clone)]
pub struct History {
    states: Vec<Arc<Document>>,
    cursor: usize,
}
impl History {
    pub fn new(doc: &Document) -> Self {
        Self {
            states: vec![Arc::new(doc.clone())],
            cursor: 0,
        }
    }
    pub fn record(&mut self, doc: &Document) {
        if self.states[self.cursor].as_ref() == doc {
            return;
        }
        self.states.truncate(self.cursor + 1);
        self.states.push(Arc::new(doc.clone()));
        if self.states.len() > 100 {
            self.states.remove(0);
        }
        self.cursor = self.states.len() - 1;
    }
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.states.len()
    }
    pub fn undo(&mut self) -> Option<Document> {
        if !self.can_undo() {
            return None;
        }
        self.cursor -= 1;
        Some(self.states[self.cursor].as_ref().clone())
    }
    pub fn redo(&mut self) -> Option<Document> {
        if !self.can_redo() {
            return None;
        }
        self.cursor += 1;
        Some(self.states[self.cursor].as_ref().clone())
    }
}
