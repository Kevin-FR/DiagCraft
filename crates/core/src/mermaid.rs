//! Original, bounded Mermaid flowchart interchange. No browser, scripts or HTML.
//! The supported grammar is specified in docs/mermaid.md before implementation.
use crate::{
    document::{Connector, ConnectorKind, Document, Port, ShapeKind},
    stroke_style::{ArrowHead, LinePattern},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_NODES: usize = 200;
pub const MAX_EDGES: usize = 400;
pub const MAX_LABEL_CHARS: usize = 4096;

#[derive(Clone, Debug, Serialize)]
pub struct Notice {
    pub code: String,
    pub severity: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub page: Option<usize>,
    pub id: Option<u64>,
    pub message: String,
    pub hint: String,
}
impl Notice {
    fn warning(
        code: &str,
        page: Option<usize>,
        id: Option<u64>,
        message: &str,
        hint: &str,
    ) -> Self {
        Self {
            code: code.into(),
            severity: "warning".into(),
            line: None,
            column: None,
            page,
            id,
            message: message.into(),
            hint: hint.into(),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct Import {
    pub document: Document,
    pub diagnostics: Vec<Notice>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Export {
    pub source: String,
    pub diagnostics: Vec<Notice>,
}
#[derive(Clone)]
struct NodeSpec {
    kind: ShapeKind,
    text: String,
}
struct EdgeSpec {
    from: usize,
    to: usize,
    pattern: LinePattern,
    thick: bool,
    arrow: bool,
    label: String,
}
#[derive(Clone, Copy)]
enum Direction {
    Tb,
    Bt,
    Lr,
    Rl,
}
struct Parser<'a> {
    source: &'a str,
    pos: usize,
    nodes: Vec<NodeSpec>,
    names: BTreeMap<String, usize>,
    edges: Vec<EdgeSpec>,
}
impl<'a> Parser<'a> {
    fn error(&self, code: &str, message: &str, hint: &str) -> Box<Notice> {
        let before = &self.source[..self.pos];
        Box::new(Notice {
            code: code.into(),
            severity: "error".into(),
            line: Some(before.bytes().filter(|b| *b == b'\n').count() + 1),
            column: Some(before.rsplit('\n').next().unwrap_or("").chars().count() + 1),
            page: None,
            id: None,
            message: message.into(),
            hint: hint.into(),
        })
    }
    fn rest(&self) -> &'a str {
        &self.source[self.pos..]
    }
    fn take(&mut self, token: &str) -> bool {
        if self.rest().starts_with(token) {
            self.pos += token.len();
            true
        } else {
            false
        }
    }
    fn horizontal(&mut self) {
        while self.rest().starts_with([' ', '\t', '\r']) {
            self.pos += 1;
        }
    }
    fn comment(&mut self) -> Result<bool, Box<Notice>> {
        if !self.rest().starts_with("%%") {
            return Ok(false);
        }
        if self.rest().starts_with("%%{") {
            return Err(self.error(
                "unsupported_directive",
                "Mermaid directives are not supported.",
                "Remove the directive and keep only a flowchart declaration, nodes and links.",
            ));
        }
        self.pos += self.rest().find('\n').unwrap_or(self.rest().len());
        Ok(true)
    }
    fn separators(&mut self) -> Result<(), Box<Notice>> {
        loop {
            self.horizontal();
            if self.take("\n") || self.take(";") {
                continue;
            }
            if self.comment()? {
                continue;
            }
            return Ok(());
        }
    }
    fn word(&mut self) -> Result<String, Box<Notice>> {
        let start = self.pos;
        let Some(c) = self.rest().chars().next() else {
            return Err(self.error(
                "expected_node",
                "A node identifier is missing.",
                "Use an ASCII identifier such as A, Step_2 or _start.",
            ));
        };
        if !c.is_ascii_alphabetic() && c != '_' {
            return Err(self.error(
                "invalid_identifier",
                "Unsupported node identifier or syntax.",
                "Use ASCII identifiers; place Unicode text inside a quoted node label.",
            ));
        }
        self.pos += 1;
        while self
            .rest()
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.pos += 1;
        }
        Ok(self.source[start..self.pos].to_owned())
    }
    fn text(&mut self, close: &str) -> Result<String, Box<Notice>> {
        self.horizontal();
        let quoted = self.take("\"");
        let start = self.pos;
        if quoted {
            while !self.rest().starts_with('"') && !self.rest().is_empty() {
                let c = self.rest().chars().next().unwrap();
                self.pos += c.len_utf8();
            }
            if self.rest().is_empty() {
                return Err(self.error(
                    "unclosed_label",
                    "The quoted label is not closed.",
                    "Close the double quote and the node or edge delimiter.",
                ));
            }
        } else {
            while !self.rest().starts_with(close) && !self.rest().is_empty() {
                let c = self.rest().chars().next().unwrap();
                if c == '"' || c == '`' || matches!(c, '[' | ']' | '(' | ')' | '{' | '}') {
                    return Err(self.error("unsupported_label", "A label contains reserved syntax.", "Use a double-quoted label and numeric entities for quotes or reserved delimiters."));
                }
                self.pos += c.len_utf8();
            }
        }
        let end = self.pos;
        if quoted {
            self.pos += 1;
            self.horizontal();
        }
        if !self.take(close) {
            return Err(self.error(
                "unclosed_label",
                "The label delimiter is not closed.",
                "Close the node shape or the edge label delimiter before continuing.",
            ));
        }
        let raw = &self.source[start..end];
        let raw = if quoted { raw } else { raw.trim() };
        decode_label(raw).map_err(|(code, message, hint)| self.error(code, message, hint))
    }
    fn node(&mut self) -> Result<usize, Box<Notice>> {
        let name = self.word()?;
        if [
            "subgraph",
            "end",
            "style",
            "class",
            "classDef",
            "linkStyle",
            "click",
            "direction",
            "flowchart",
            "graph",
            "accTitle",
            "accDescr",
        ]
        .contains(&name.as_str())
        {
            return Err(self.error("unsupported_construct", "This Mermaid construct is outside the supported subset.", "Keep plain node declarations and simple links; remove styles, subgraphs and interactive directives."));
        }
        self.horizontal();
        let shape = [
            ("([", "])", ShapeKind::Terminator),
            ("((", "))", ShapeKind::Ellipse),
            ("[[", "]]", ShapeKind::PredefinedProcess),
            ("[(", ")]", ShapeKind::Database),
            ("{{", "}}", ShapeKind::Hexagon),
            ("[", "]", ShapeKind::Rectangle),
            ("(", ")", ShapeKind::Rounded),
            ("{", "}", ShapeKind::Diamond),
        ]
        .into_iter()
        .find(|(open, _, _)| self.rest().starts_with(open));
        let explicit = if let Some((open, close, kind)) = shape {
            self.pos += open.len();
            Some((kind, self.text(close)?))
        } else {
            None
        };
        if let Some(index) = self.names.get(&name).copied() {
            if let Some((kind, text)) = explicit {
                self.nodes[index].kind = kind;
                self.nodes[index].text = text;
            }
            return Ok(index);
        }
        if self.nodes.len() >= MAX_NODES {
            return Err(self.error(
                "node_limit",
                "The flowchart exceeds 200 nodes.",
                "Split the flowchart into smaller documents.",
            ));
        }
        let index = self.nodes.len();
        let (kind, text) = explicit.unwrap_or((ShapeKind::Rectangle, name.clone()));
        self.names.insert(name.clone(), index);
        self.nodes.push(NodeSpec { kind, text });
        Ok(index)
    }
    fn parse(mut self) -> Result<(Direction, Vec<NodeSpec>, Vec<EdgeSpec>), Box<Notice>> {
        self.separators()?;
        let kind = self.word()?;
        if kind != "flowchart" && kind != "graph" {
            return Err(self.error(
                "unsupported_diagram",
                "Only Mermaid flowcharts are supported.",
                "Start with flowchart TB, TD, BT, LR or RL; other diagram types are not imported.",
            ));
        }
        self.horizontal();
        let dir = match self.word()?.as_str() {
            "TB" | "TD" => Direction::Tb,
            "BT" => Direction::Bt,
            "LR" => Direction::Lr,
            "RL" => Direction::Rl,
            _ => {
                return Err(self.error(
                    "unsupported_direction",
                    "The flowchart direction is unsupported.",
                    "Use TB, TD, BT, LR or RL.",
                ));
            }
        };
        self.horizontal();
        if !self.rest().is_empty()
            && !self.rest().starts_with(['\n', ';'])
            && !self.rest().starts_with("%%")
        {
            return Err(self.error(
                "header_separator",
                "Separate the flowchart header from its nodes.",
                "Add a newline or semicolon after the flowchart direction.",
            ));
        }
        self.separators()?;
        while !self.rest().is_empty() {
            let mut from = self.node()?;
            loop {
                self.horizontal();
                let edge = if self.take("-.->") {
                    Some((LinePattern::Dotted, false, true))
                } else if self.take("-->") {
                    Some((LinePattern::Solid, false, true))
                } else if self.take("---") {
                    Some((LinePattern::Solid, false, false))
                } else if self.take("==>") {
                    Some((LinePattern::Solid, true, true))
                } else {
                    None
                };
                let Some((pattern, thick, arrow)) = edge else {
                    break;
                };
                self.horizontal();
                let label = if self.take("|") {
                    self.text("|")?
                } else {
                    String::new()
                };
                self.horizontal();
                let to = self.node()?;
                if from == to {
                    return Err(self.error("self_link", "Attached self-links cannot be represented by DiagCraft.", "Replace the self-link with a separate step, or draw a free connector manually after import."));
                }
                if self.edges.len() >= MAX_EDGES {
                    return Err(self.error(
                        "edge_limit",
                        "The flowchart exceeds 400 links.",
                        "Split the flowchart into smaller documents.",
                    ));
                }
                // Connector labels have a stricter byte budget in the v7 model.
                if label.len() > 4096 {
                    return Err(self.error(
                        "edge_label_limit",
                        "The edge label exceeds the document byte limit.",
                        "Shorten the link label to at most 4096 UTF-8 bytes.",
                    ));
                }
                self.edges.push(EdgeSpec {
                    from,
                    to,
                    pattern,
                    thick,
                    arrow,
                    label,
                });
                from = to;
            }
            self.horizontal();
            if !self.rest().is_empty()
                && !self.rest().starts_with(['\n', ';'])
                && !self.rest().starts_with("%%")
            {
                return Err(self.error("unsupported_syntax", "Unsupported Mermaid syntax follows this node or link.", "Use simple shape declarations and -->, ---, -.-> or ==> links; remove unsupported directives or extended syntax."));
            }
            self.separators()?;
        }
        if self.nodes.is_empty() {
            return Err(self.error(
                "empty_graph",
                "The flowchart contains no nodes.",
                "Add at least one node, for example A[Start].",
            ));
        }
        Ok((dir, self.nodes, self.edges))
    }
}
fn decode_label(raw: &str) -> Result<String, (&'static str, &'static str, &'static str)> {
    let mut rest = raw;
    let mut text = String::new();
    let mut count = 0;
    while !rest.is_empty() {
        if count >= MAX_LABEL_CHARS {
            return Err((
                "label_limit",
                "A label exceeds 4096 characters.",
                "Shorten the label or split the text into separate nodes.",
            ));
        }
        count += 1;
        if rest.starts_with('<') {
            let br = ["<br>", "<br/>", "<br />"]
                .into_iter()
                .find(|t| rest.starts_with(t));
            if let Some(br) = br {
                text.push('\n');
                rest = &rest[br.len()..];
                continue;
            }
            return Err((
                "unsupported_html",
                "Only line-break tags are supported in labels.",
                "Replace HTML with plain text or numeric entities; use <br/> for a line break.",
            ));
        }
        if rest.starts_with('`') {
            return Err((
                "unsupported_markdown",
                "Markdown labels are not supported.",
                "Use plain quoted text without backticks.",
            ));
        }
        let entity = rest
            .strip_prefix("&#")
            .or_else(|| rest.strip_prefix('#'))
            .or_else(|| rest.strip_prefix('&'));
        if let Some(entity) = entity
            && let Some(end) = entity.as_bytes().iter().take(12).position(|b| *b == b';')
            && !entity[..end].is_empty()
            && entity[..end].bytes().all(|b| b.is_ascii_alphanumeric())
            && !entity[..end].bytes().all(|b| b.is_ascii_digit())
        {
            return Err((
                "unsupported_entity",
                "Only decimal numeric entities are supported.",
                "Replace named entities with numeric entities such as #34; and #38;.",
            ));
        }
        let numeric = if rest.starts_with("&#") {
            Some(2)
        } else if rest.starts_with('#') && rest.as_bytes().get(1).is_some_and(u8::is_ascii_digit) {
            Some(1)
        } else {
            None
        };
        if let Some(prefix) = numeric {
            let Some(end) = rest.as_bytes().iter().take(12).position(|b| *b == b';') else {
                return Err((
                    "invalid_entity",
                    "The numeric entity is not terminated.",
                    "Use decimal numeric entities such as #34; or #60;.",
                ));
            };
            let digits = &rest[prefix..end];
            if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err((
                    "invalid_entity",
                    "Invalid numeric entity in a label.",
                    "Use decimal numeric entities such as #34; or #60;.",
                ));
            }
            let c = digits
                .parse::<u32>()
                .ok()
                .and_then(char::from_u32)
                .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
                .ok_or((
                    "invalid_entity",
                    "The numeric entity is not a supported Unicode scalar.",
                    "Use a valid decimal Unicode code point; control characters are not supported.",
                ))?;
            text.push(c);
            rest = &rest[end + 1..];
        } else {
            let c = rest.chars().next().unwrap();
            if c.is_control() && !matches!(c, '\n' | '\r' | '\t') {
                return Err((
                    "invalid_label",
                    "A label contains a control character.",
                    "Remove control characters and keep plain Unicode text.",
                ));
            }
            text.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Parse the whole source before constructing an editable single-page document.
pub fn import(source: &str) -> Result<Import, Vec<Notice>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(vec![Notice {
            code: "source_limit".into(),
            severity: "error".into(),
            line: None,
            column: None,
            page: None,
            id: None,
            message: "Mermaid source exceeds 256 KiB.".into(),
            hint: "Split the source into smaller flowcharts.".into(),
        }]);
    }
    let parser = Parser {
        source,
        pos: 0,
        nodes: vec![],
        names: BTreeMap::new(),
        edges: vec![],
    };
    let (dir, nodes, edges) = parser.parse().map_err(|e| vec![*e])?;
    let mut document = Document {
        title: "Mermaid flowchart".into(),
        ..Default::default()
    };
    let positions = layout(dir, &nodes, &edges);
    let mut ids = Vec::new();
    for (spec, position) in nodes.iter().zip(positions) {
        let id = document.add_node(spec.kind, position);
        let n = document.node_mut(id).unwrap();
        n.position = position;
        n.text = spec.text.clone();
        n.size = node_size(spec.kind);
        n.font_size = 16.;
        n.text_style.wrap = true;
        ids.push(id);
    }
    for edge in edges {
        let id = document.next_id();
        document.connectors.push(Connector {
            id,
            from: ids[edge.from],
            to: ids[edge.to],
            free_start: None,
            free_end: None,
            kind: ConnectorKind::Elbow,
            label: edge.label,
            color: [111, 128, 139],
            width: if edge.thick { 4. } else { 1.8 },
            arrow: edge.arrow,
            arrow_start: false,
            from_port: Port::Auto,
            to_port: Port::Auto,
            waypoints: vec![],
            label_t: 0.5,
            pattern: edge.pattern,
            start_head: ArrowHead::Triangle,
            end_head: ArrowHead::Triangle,
            appearance: Default::default(),
        });
    }
    document.page_size = [
        document
            .nodes
            .iter()
            .map(|n| n.position[0] + n.size[0] + 60.)
            .fold(100., f32::max),
        document
            .nodes
            .iter()
            .map(|n| n.position[1] + n.size[1] + 60.)
            .fold(100., f32::max),
    ];
    document.validate().map_err(|e| {
        vec![Notice {
            code: "invalid_document".into(),
            severity: "error".into(),
            line: None,
            column: None,
            page: None,
            id: None,
            message: e,
            hint: "Reduce the flowchart or its label sizes.".into(),
        }]
    })?;
    Ok(Import {
        document,
        diagnostics: vec![Notice::warning(
            "layout_changed",
            Some(0),
            None,
            "DiagCraft generates an editable layout from the flowchart structure.",
            "Review node positions and connector routes before applying; Mermaid's visual layout is not preserved.",
        )],
    })
}
fn node_size(kind: ShapeKind) -> [f32; 2] {
    match kind {
        ShapeKind::Ellipse => [120., 120.],
        ShapeKind::Diamond => [220., 140.],
        _ => [220., 100.],
    }
}
/// Collapse strongly connected components via bounded reachability, then rank the
/// resulting DAG. At most 200^3 booleans are visited, with no recursive traversal.
fn layout(dir: Direction, nodes: &[NodeSpec], edges: &[EdgeSpec]) -> Vec<[f32; 2]> {
    let n = nodes.len();
    let mut reachable = vec![vec![false; n]; n];
    for (i, row) in reachable.iter_mut().enumerate() {
        row[i] = true;
    }
    for e in edges {
        reachable[e.from][e.to] = true;
    }
    for k in 0..n {
        let via = reachable[k].clone();
        for row in &mut reachable {
            if row[k] {
                for (cell, &through) in row.iter_mut().zip(&via) {
                    *cell |= through;
                }
            }
        }
    }
    let mut group = vec![usize::MAX; n];
    let mut count = 0;
    for i in 0..n {
        if group[i] == usize::MAX {
            for j in i..n {
                if reachable[i][j] && reachable[j][i] {
                    group[j] = count;
                }
            }
            count += 1;
        }
    }
    let mut next = vec![BTreeSet::new(); count];
    let mut indegree = vec![0usize; count];
    for e in edges {
        let (a, b) = (group[e.from], group[e.to]);
        if a != b && next[a].insert(b) {
            indegree[b] += 1;
        }
    }
    let mut queue: VecDeque<_> = (0..count).filter(|i| indegree[*i] == 0).collect();
    let mut ranks = vec![0usize; count];
    while let Some(a) = queue.pop_front() {
        for &b in &next[a] {
            ranks[b] = ranks[b].max(ranks[a] + 1);
            indegree[b] -= 1;
            if indegree[b] == 0 {
                queue.push_back(b);
            }
        }
    }
    let mut slots = BTreeMap::<usize, usize>::new();
    let max_rank = *ranks.iter().max().unwrap_or(&0);
    nodes
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let rank = ranks[group[i]];
            let slot = slots.entry(rank).or_default();
            let main = if matches!(dir, Direction::Bt | Direction::Rl) {
                max_rank - rank
            } else {
                rank
            };
            let pos = if matches!(dir, Direction::Tb | Direction::Bt) {
                [60. + *slot as f32 * 300., 60. + main as f32 * 220.]
            } else {
                [60. + main as f32 * 320., 60. + *slot as f32 * 220.]
            };
            *slot += 1;
            pos
        })
        .collect()
}
fn validate_export_label(
    label: &str,
    id: u64,
    diagnostics: &mut Vec<Notice>,
    page: usize,
) -> Result<(), String> {
    if label
        .chars()
        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
    {
        return Err(format!(
            "Object {id} contains control characters unsupported by Mermaid; remove them before exporting."
        ));
    }
    if label.contains('\r') {
        diagnostics.push(Notice::warning(
            "label_normalized",
            Some(page),
            Some(id),
            "Carriage-return line endings become newline characters.",
            "Keep the .draw file to retain the exact original string representation.",
        ));
    }
    Ok(())
}
fn encode_label(label: &str) -> String {
    let mut out = String::new();
    let normalized = label.replace("\r\n", "\n").replace('\r', "\n");
    for c in normalized.chars() {
        match c {
            '\n' => out.push_str("<br/>"),
            '\r' => {}
            '"' | '#' | '&' | '<' | '>' | '[' | ']' | '(' | ')' | '{' | '}' | '|' | '`' | ';'
            | '\\' => {
                out.push('#');
                out.push_str(&(c as u32).to_string());
                out.push(';');
            }
            _ => out.push(c),
        }
    }
    out
}
/// Export one chosen page only. Callers must present/accept diagnostics before writing.
pub fn export(doc: &Document, page: usize) -> Result<Export, String> {
    doc.validate()?;
    let d = doc.page_document(page).ok_or("Mermaid page not found.")?;
    let view = d.organization_view();
    let mut diagnostics = vec![Notice::warning(
        "layout_changed",
        Some(page),
        None,
        "Mermaid export does not preserve coordinates, sizes, routes, ports, styles or page appearance.",
        "Keep the .draw file as the editable visual source; review the Mermaid preview before writing.",
    )];
    if !d.organization.is_empty() {
        diagnostics.push(Notice::warning(
            "organization_flattened",
            Some(page),
            None,
            "Visible layers and groups are flattened; hidden objects are omitted.",
            "Keep the .draw file to preserve groups, layers, locks and hidden content.",
        ));
    }
    let visible: Vec<_> = d
        .nodes
        .iter()
        .filter(|n| view.is_object_visible(n.id))
        .collect();
    let ids: BTreeSet<_> = visible.iter().map(|n| n.id).collect();
    if visible.is_empty() {
        return Err("The chosen page has no visible nodes to export as a flowchart.".into());
    }
    if visible.len() > MAX_NODES {
        return Err("Mermaid export exceeds 200 visible nodes; select a smaller page.".into());
    }
    let mut source = String::from("flowchart TB\n");
    for n in visible {
        let (open, close) = match n.kind {
            ShapeKind::Rectangle => ("[", "]"),
            ShapeKind::Rounded => ("(", ")"),
            ShapeKind::Terminator => ("([", "])"),
            ShapeKind::Diamond => ("{", "}"),
            ShapeKind::Ellipse => ("((", "))"),
            ShapeKind::Hexagon => ("{{", "}}"),
            ShapeKind::PredefinedProcess => ("[[", "]]"),
            ShapeKind::Database => ("[(", ")]"),
            _ => {
                diagnostics.push(Notice::warning(
                    "shape_converted",
                    Some(page),
                    Some(n.id),
                    "This node shape is exported as a rectangle.",
                    "Use a supported flowchart shape if the distinction is important.",
                ));
                ("[", "]")
            }
        };
        let label = if n.text.is_empty() {
            n.image.as_ref().map(|i| i.alt.as_str()).unwrap_or(&n.text)
        } else {
            &n.text
        };
        validate_export_label(label, n.id, &mut diagnostics, page)?;
        if label.chars().count() > MAX_LABEL_CHARS {
            return Err(format!(
                "Node {} exceeds Mermaid's 4096-character label limit.",
                n.id
            ));
        }
        if n.image.is_some() {
            diagnostics.push(Notice::warning(
                "image_omitted",
                Some(page),
                Some(n.id),
                "Embedded pixels and crop are not represented in Mermaid.",
                "Keep the .draw file or use SVG, PNG or PDF to share the image.",
            ));
        }
        if !n.text_runs.is_empty() {
            diagnostics.push(Notice::warning(
                "rich_text_flattened",
                Some(page),
                Some(n.id),
                "Rich text is exported as plain text.",
                "Keep the .draw file to preserve character formatting.",
            ));
        }
        source.push_str(&format!(
            "  n{}{open}\"{}\"{close}\n",
            n.id,
            encode_label(label)
        ));
    }
    for c in &d.charts {
        if view.is_object_visible(c.id) {
            diagnostics.push(Notice::warning(
                "chart_omitted",
                Some(page),
                Some(c.id),
                "A chart is omitted from the flowchart export.",
                "Export SVG, PNG or PDF to preserve chart data visually.",
            ));
        }
    }
    for c in &d.drawings {
        if view.is_object_visible(c.id) {
            diagnostics.push(Notice::warning(
                "drawing_omitted",
                Some(page),
                Some(c.id),
                "A free drawing is omitted from the flowchart export.",
                "Export SVG, PNG or PDF to preserve free drawings.",
            ));
        }
    }
    let mut count = 0;
    for e in &d.connectors {
        if !view.is_object_visible(e.id) {
            continue;
        }
        if !ids.contains(&e.from) || !ids.contains(&e.to) {
            diagnostics.push(Notice::warning(
                "connector_omitted",
                Some(page),
                Some(e.id),
                "A connector with a free or unavailable endpoint is omitted.",
                "Attach both endpoints to visible nodes before exporting.",
            ));
            continue;
        }
        count += 1;
        if count > MAX_EDGES {
            return Err("Mermaid export exceeds 400 links; select a smaller page.".into());
        }
        validate_export_label(&e.label, e.id, &mut diagnostics, page)?;
        if e.label.chars().count() > MAX_LABEL_CHARS {
            return Err(format!("Connector {} exceeds Mermaid's label limit.", e.id));
        }
        if e.arrow_start || (e.arrow && e.end_head != ArrowHead::Triangle) {
            diagnostics.push(Notice::warning(
                "arrow_converted",
                Some(page),
                Some(e.id),
                "Arrowhead decorations are reduced to a simple end arrow or undirected line.",
                "Keep the .draw file or use a visual export to preserve arrowheads.",
            ));
        }
        let arrow = if !e.arrow {
            "---"
        } else if e.pattern != LinePattern::Solid {
            "-.->"
        } else if e.width >= 3. {
            "==>"
        } else {
            "-->"
        };
        if e.arrow && e.pattern == LinePattern::Dashed {
            diagnostics.push(Notice::warning(
                "line_converted",
                Some(page),
                Some(e.id),
                "A dashed arrow becomes Mermaid's dotted arrow.",
                "Keep the .draw file to preserve exact dash spacing.",
            ));
        }
        if !e.arrow && e.pattern != LinePattern::Solid {
            diagnostics.push(Notice::warning(
                "line_converted",
                Some(page),
                Some(e.id),
                "An undirected patterned link becomes a solid undirected line.",
                "Use an end arrow to retain a dotted pattern in this subset.",
            ));
        }
        let label = if e.label.is_empty() {
            String::new()
        } else {
            format!("|\"{}\"|", encode_label(&e.label))
        };
        source.push_str(&format!("  n{} {arrow}{label} n{}\n", e.from, e.to));
    }
    if source.len() > MAX_SOURCE_BYTES {
        return Err("Mermaid export exceeds 256 KiB after escaping; shorten the labels.".into());
    }
    Ok(Export {
        source,
        diagnostics,
    })
}
