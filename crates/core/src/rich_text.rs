//! Inline formatting uses Unicode scalar offsets into the original plain text.
use crate::document::{Color, Node, TextStyle};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_RUNS: usize = 1024;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct InlineStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underline: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strikethrough: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monospace: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
}
impl InlineStyle {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
    fn overlay(&mut self, style: &Self) {
        macro_rules! fields { ($($field:ident),+) => { $(if style.$field.is_some() { self.$field = style.$field; })+ }; }
        fields!(
            bold,
            italic,
            underline,
            strikethrough,
            monospace,
            font_size,
            color
        );
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRun {
    pub start: usize,
    pub end: usize,
    pub style: InlineStyle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedStyle {
    pub text_style: TextStyle,
    pub font_size: f32,
    pub color: Color,
}

impl Node {
    pub fn inline_style_at(&self, index: usize) -> InlineStyle {
        let at = self.text_runs.partition_point(|run| run.end <= index);
        self.text_runs
            .get(at)
            .filter(|r| r.start <= index)
            .map(|r| r.style.clone())
            .unwrap_or_default()
    }

    pub fn resolved_style_at(&self, index: usize) -> ResolvedStyle {
        let inline = self.inline_style_at(index);
        let mut style = self.text_style.clone();
        macro_rules! fields { ($($field:ident),+) => { $(if let Some(value) = inline.$field { style.$field = value; })+ }; }
        fields!(bold, italic, underline, strikethrough, monospace);
        ResolvedStyle {
            text_style: style,
            font_size: inline.font_size.unwrap_or(self.font_size),
            color: inline.color.unwrap_or(self.text_color),
        }
    }

    pub fn validate_text_runs(&self) -> Result<(), String> {
        if self.text_runs.len() > MAX_RUNS {
            return Err("Trop de portions de texte mises en forme (maximum 1024).".into());
        }
        let mut boundaries = vec![0];
        let mut count = 0;
        for grapheme in self.text.graphemes(true) {
            count += grapheme.chars().count();
            boundaries.push(count);
        }
        let mut previous_end = 0;
        for run in &self.text_runs {
            if run.start < previous_end
                || run.start >= run.end
                || run.end > count
                || boundaries.binary_search(&run.start).is_err()
                || boundaries.binary_search(&run.end).is_err()
                || run
                    .style
                    .font_size
                    .is_some_and(|v| !v.is_finite() || !(6.0..=200.).contains(&v))
            {
                return Err("Portion de texte invalide : plages ordonnées, sans chevauchement, aux frontières de graphèmes ; taille entre 6 et 200 px.".into());
            }
            previous_end = run.end;
        }
        Ok(())
    }

    fn runs_from_styles(text: &str, styles: &[InlineStyle]) -> Result<Vec<TextRun>, String> {
        let mut runs: Vec<TextRun> = vec![];
        let mut start = 0;
        for grapheme in text.graphemes(true) {
            let end = start + grapheme.chars().count();
            let style = styles.get(start).cloned().unwrap_or_default();
            if !style.is_empty() {
                if let Some(last) = runs
                    .last_mut()
                    .filter(|r| r.end == start && r.style == style)
                {
                    last.end = end;
                } else {
                    runs.push(TextRun { start, end, style });
                }
            }
            start = end;
        }
        if runs.len() > MAX_RUNS {
            return Err("Trop de portions de texte mises en forme (maximum 1024).".into());
        }
        Ok(runs)
    }

    /// Apply a partial style to the selection, expanding to whole graphemes.
    /// None removes overrides in the selection and restores the block's base style.
    pub fn format_text(
        &mut self,
        selection: Range<usize>,
        style: Option<&InlineStyle>,
    ) -> Result<(), String> {
        self.validate_text_runs()?;
        let count = self.text.chars().count();
        if selection.start >= selection.end || selection.end > count {
            return Err("Sélection de texte invalide.".into());
        }
        let mut styles: Vec<_> = (0..count).map(|i| self.inline_style_at(i)).collect();
        let mut start = 0;
        for grapheme in self.text.graphemes(true) {
            let end = start + grapheme.chars().count();
            if start < selection.end && end > selection.start {
                for current in &mut styles[start..end] {
                    if let Some(style) = style {
                        current.overlay(style);
                    } else {
                        *current = InlineStyle::default();
                    }
                }
            }
            start = end;
        }
        self.text_runs = Self::runs_from_styles(&self.text, &styles)?;
        Ok(())
    }

    /// Keep formatting outside the changed range; inserted text inherits at the caret.
    /// Offsets are never interpreted as UTF-8 bytes, including for CJK and emoji.
    pub fn set_text(&mut self, text: String) -> Result<(), String> {
        if text.len() > 65_536 {
            return Err("Texte trop volumineux (maximum 65 536 octets).".into());
        }
        if text == self.text {
            return Ok(());
        }
        let old: Vec<_> = self.text.chars().collect();
        let new: Vec<_> = text.chars().collect();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        let suffix = old[prefix..]
            .iter()
            .rev()
            .zip(new[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let inherited =
            self.inline_style_at(if prefix < old.len() && prefix + suffix < old.len() {
                prefix
            } else {
                prefix.saturating_sub(1)
            });
        let mut styles: Vec<_> = (0..prefix).map(|i| self.inline_style_at(i)).collect();
        styles.extend(std::iter::repeat_n(inherited, new.len() - prefix - suffix));
        styles.extend((old.len() - suffix..old.len()).map(|i| self.inline_style_at(i)));
        let runs = Self::runs_from_styles(&text, &styles)?;
        self.text = text;
        self.text_runs = runs;
        Ok(())
    }
}
