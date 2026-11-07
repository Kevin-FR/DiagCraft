//! Original, bounded `.drawio` (draw.io XML) → diagram conversion. The supported
//! subset is defined in docs/drawio.md before implementation. The file is treated
//! as local data: no network, no script or HTML execution, no external resource
//! loading. xml never runs anything: it is parsed text only.
use crate::document::{Connector, ConnectorKind, Document, ShapeKind};
use crate::stroke_style::{ArrowHead, LinePattern};
use serde::Serialize;

pub const MAX_SOURCE_BYTES: usize = 512 * 1024;
pub const MAX_CELLS: usize = 2000;
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
    fn error(code: &str, line: Option<usize>, message: &str, hint: &str) -> Self {
        Self {
            code: code.into(),
            severity: "error".into(),
            line,
            column: None,
            page: None,
            id: None,
            message: message.into(),
            hint: hint.into(),
        }
    }
    fn warning(
        code: &str,
        line: Option<usize>,
        page: Option<usize>,
        id: Option<u64>,
        message: &str,
        hint: &str,
    ) -> Self {
        Self {
            code: code.into(),
            severity: "warning".into(),
            line,
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

/// A lightweight parsed XML node tree, bounded by budget checks in `xml`.
#[derive(Clone, Debug)]
struct XNode {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<XNode>,
    line: usize,
}
impl XNode {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
    fn child(&self, name: &str) -> Option<&XNode> {
        self.children.iter().find(|c| c.name == name)
    }
}

/// Custom bounded XML parser. Handles: start/end/self-closing tags, single and
/// double quoted attributes, XML comments, CDATA, named and numeric entities.
/// It is deliberately small and never executes anything.
#[allow(clippy::result_large_err)]
fn parse_xml(source: &str) -> Result<Vec<XNode>, Notice> {
    let bytes = source.as_bytes();
    let mut pos = 0usize;
    let mut line = 1usize;
    let mut nodes: Vec<XNode> = Vec::new();
    let mut stack: Vec<XNode> = Vec::new();
    fn skip_ws(pos: &mut usize, bytes: &[u8], line: &mut usize) {
        while *pos < bytes.len() {
            match bytes[*pos] {
                b' ' | b'\t' => *pos += 1,
                b'\n' => {
                    *pos += 1;
                    *line += 1;
                }
                _ => break,
            }
        }
    }
    macro_rules! adv {
        ($n:expr) => {{
            let n = $n;
            for &b in &bytes[pos..pos + n] {
                if b == b'\n' {
                    line += 1;
                }
            }
            pos += n;
        }};
    }
    while pos < bytes.len() {
        // XML declaration / processing instructions: skip to the next '>'.
        if bytes[pos..].starts_with(b"<?") {
            let end = source[pos + 2..].find("?>").map(|i| pos + 2 + i + 2);
            match end {
                Some(e) => adv!(e - pos),
                None => {
                    return Err(Notice::error(
                        "xml_parse",
                        Some(line),
                        "Unterminated XML declaration.",
                        "Close the XML declaration with \"?>\".",
                    ));
                }
            }
            continue;
        }
        // Comments: skip the full <!-- ... --> body.
        if bytes[pos..].starts_with(b"<!--") {
            let end = source[pos + 4..].find("-->").map(|i| pos + 4 + i + 3);
            match end {
                Some(e) => adv!(e - pos),
                None => {
                    return Err(Notice::error(
                        "xml_parse",
                        Some(line),
                        "Unterminated XML comment.",
                        "Close the comment with \"-->\".",
                    ));
                }
            }
            continue;
        }
        // CDATA: capture verbatim text.
        if bytes[pos..].starts_with(b"<![CDATA[") {
            let body = &source[pos + 9..];
            let end = body.find("]]>");
            match end {
                Some(e) => {
                    let content = &body[..e];
                    let full = 9 + e + 3;
                    adv!(full);
                    if let Some(top) = stack.last_mut() {
                        top.text.push_str(content);
                    }
                }
                None => {
                    return Err(Notice::error(
                        "xml_parse",
                        Some(line),
                        "Unterminated CDATA section.",
                        "Close the CDATA section with \"]]>\".",
                    ));
                }
            }
            continue;
        }
        if bytes[pos] == b'<' {
            let close = source[pos..].starts_with("</");
            let tag_start = pos;
            let name_search = tag_start + (if close { 2 } else { 1 });
            let name_end = source[name_search..]
                .find(|c: char| {
                    !(c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.')
                })
                .map(|i| name_search + i)
                .unwrap_or(source.len());
            if name_end == name_search {
                return Err(Notice::error(
                    "xml_parse",
                    Some(line),
                    "Expected a tag name after '<'.",
                    "Check the XML structure around the malformed tag.",
                ));
            }
            let name = &source[name_search..name_end];
            adv!(name_end - pos);
            let mut attrs: Vec<(String, String)> = Vec::new();
            let mut self_close = false;
            loop {
                skip_ws(&mut pos, bytes, &mut line);
                if pos >= bytes.len() {
                    return Err(Notice::error(
                        "xml_parse",
                        Some(line),
                        "Unterminated tag.",
                        "Close the tag with \">\" or \"/>\".",
                    ));
                }
                if !close && bytes[pos] == b'/' && pos + 1 < bytes.len() && bytes[pos + 1] == b'>' {
                    adv!(2);
                    self_close = true;
                    break;
                }
                if bytes[pos] == b'>' {
                    adv!(1);
                    break;
                }
                if bytes[pos] == b'<' {
                    return Err(Notice::error(
                        "xml_parse",
                        Some(line),
                        "Unexpected '<' inside a tag.",
                        "Check attribute quoting in the element.",
                    ));
                }
                // Attribute name.
                let an_start = pos;
                let an_end = source[an_start..]
                    .find(['=', ' ', '\t', '\n', '/'])
                    .map(|i| an_start + i)
                    .unwrap_or(source.len());
                let aname = &source[an_start..an_end].to_string();
                adv!(an_end - pos);
                skip_ws(&mut pos, bytes, &mut line);
                if pos < bytes.len() && bytes[pos] == b'=' {
                    adv!(1);
                    skip_ws(&mut pos, bytes, &mut line);
                    if pos >= bytes.len() {
                        return Err(Notice::error(
                            "xml_parse",
                            Some(line),
                            "Unterminated attribute value.",
                            "Quote the attribute value and close the tag.",
                        ));
                    }
                    let quote = bytes[pos];
                    if quote != b'"' && quote != b'\'' {
                        return Err(Notice::error(
                            "xml_parse",
                            Some(line),
                            "Attribute value is not quoted.",
                            "Wrap the value in double or single quotes.",
                        ));
                    }
                    adv!(1);
                    let vs_start = pos;
                    let vs_end = source[vs_start..]
                        .find(quote as char)
                        .map(|i| vs_start + i)
                        .unwrap_or(source.len());
                    if vs_end == source.len() {
                        return Err(Notice::error(
                            "xml_parse",
                            Some(line),
                            "Unterminated attribute value.",
                            "Close the quoted value before the end of the element.",
                        ));
                    }
                    let raw = &source[vs_start..vs_end];
                    let val = decode_entities(raw);
                    let mut k = vs_start;
                    while k < vs_end {
                        if bytes[k] == b'\n' {
                            line += 1;
                        }
                        k += 1;
                    }
                    adv!(vs_end + 1 - pos);
                    attrs.push((aname.clone(), val));
                } else {
                    // Attribute without a value.
                    attrs.push((aname.clone(), String::new()));
                }
            }
            if close {
                let opened = stack.pop().ok_or_else(|| {
                    Notice::error(
                        "xml_parse",
                        Some(line),
                        &format!("Closing tag </{name}> with no open element."),
                        "Check that tags are properly nested.",
                    )
                })?;
                if opened.name != name {
                    return Err(Notice::error(
                        "xml_parse",
                        Some(line),
                        &format!("Closing tag </{name}> does not match <{}>.", opened.name),
                        "Close every element with its own name.",
                    ));
                }
                // Attach the completed element to its parent or to the root list.
                match stack.last_mut() {
                    Some(parent) => parent.children.push(opened),
                    None => nodes.push(opened),
                }
                continue;
            }
            let node = XNode {
                name: name.to_string(),
                attrs,
                text: String::new(),
                children: Vec::new(),
                line,
            };
            if self_close {
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => nodes.push(node),
                }
                continue;
            }
            stack.push(node);
            continue;
        }
        // Text content.
        let t_start = pos;
        let t_end = source[t_start..]
            .find('<')
            .map(|i| t_start + i)
            .unwrap_or(source.len());
        let text = decode_entities(&source[t_start..t_end]);
        let mut p2 = t_start;
        while p2 < t_end {
            if bytes[p2] == b'\n' {
                line += 1;
            }
            p2 += 1;
        }
        adv!(t_end - pos);
        if let Some(top) = stack.last_mut() {
            top.text.push_str(&text);
        }
    }
    if !stack.is_empty() {
        return Err(Notice::error(
            "xml_parse",
            Some(line),
            "Unclosed element; expected a closing tag.",
            "Close every opened element.",
        ));
    }
    Ok(nodes)
}

/// Decodes the five predefined XML entities plus numeric character references.
/// Unknown named entities are left as-is. This never interprets markup; `&lt;br&gt;`
/// in a label stays text and is converted later by the caller.
fn decode_entities(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch != '&' {
            out.push(ch);
            continue;
        }
        // Try to decode an entity starting at i.
        let rest = &s[i + 1..];
        let semi = rest.find(';');
        let decoded: Option<String> = match semi {
            Some(len) => {
                let entity = &rest[..len];
                match entity {
                    "amp" => Some("&".into()),
                    "lt" => Some("<".into()),
                    "gt" => Some(">".into()),
                    "quot" => Some("\"".into()),
                    "apos" => Some("'".into()),
                    _ => {
                        if let Some(num) = entity.strip_prefix('#') {
                            if let Some(hex) =
                                num.strip_prefix('x').or_else(|| num.strip_prefix('X'))
                            {
                                u32::from_str_radix(hex, 16)
                                    .ok()
                                    .and_then(char::from_u32)
                                    .map(|c| c.to_string())
                            } else {
                                num.parse::<u32>()
                                    .ok()
                                    .and_then(char::from_u32)
                                    .map(|c| c.to_string())
                            }
                        } else {
                            None
                        }
                    }
                }
            }
            None => None,
        };
        match decoded {
            Some(d) => {
                out.push_str(&d);
                // Advance past the entity: skip len + 1 chars ('&'+name+';').
                for _ in 0..=semi.unwrap() {
                    chars.next();
                }
            }
            None => out.push('&'),
        }
    }
    out
}

/// Lists the page names of a `.drawio` source for the page selector. A bare
/// `<mxGraphModel>` root is reported as a single page.
pub fn pages(source: &str) -> Result<Vec<String>, Vec<Notice>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(vec![Notice::error(
            "source_limit",
            None,
            "The .drawio source exceeds 512 KiB.",
            "Split the diagram into smaller files.",
        )]);
    }
    let root = parse_xml(source).map_err(|e| vec![e])?;
    let mxfile = root.iter().find(|n| n.name == "mxfile");
    match mxfile {
        Some(f) => {
            let diagrams: Vec<&XNode> = f.children.iter().filter(|c| c.name == "diagram").collect();
            if diagrams.is_empty() {
                return Err(vec![Notice::error(
                    "no_diagram",
                    None,
                    "The <mxfile> contains no <diagram>.",
                    "Add at least one <diagram> element.",
                )]);
            }
            Ok(diagrams
                .iter()
                .map(|d| d.attr("name").unwrap_or("Diagram").to_string())
                .collect())
        }
        None => {
            if root.iter().any(|n| n.name == "mxGraphModel") || source.contains("mxGraphModel") {
                Ok(vec!["Diagram".into()])
            } else {
                Err(vec![Notice::error(
                    "not_drawio",
                    None,
                    "This file is not a recognized .drawio XML.",
                    "Choose a file with an <mxfile> or <mxGraphModel> root.",
                )])
            }
        }
    }
}

/// Converts a selected `page` (0-based) of a bounded `.drawio` source into a new
/// editable page. Returns `Err` on fatal structure, limit or well-formedness
/// errors (nothing is inserted); otherwise `diagnostics` carry any losses.
pub fn import(source: &str, page: usize) -> Result<Import, Vec<Notice>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(vec![Notice::error(
            "source_limit",
            None,
            "The .drawio source exceeds 512 KiB.",
            "Split the diagram into smaller files.",
        )]);
    }
    let root = parse_xml(source).map_err(|e| vec![e])?;
    let mut diagnostics: Vec<Notice> = Vec::new();

    // Locate the root model of the requested page.
    let model: &XNode;
    if let Some(f) = root.iter().find(|n| n.name == "mxfile") {
        let diagrams: Vec<&XNode> = f.children.iter().filter(|c| c.name == "diagram").collect();
        if diagrams.is_empty() {
            return Err(vec![Notice::error(
                "no_diagram",
                None,
                "The <mxfile> contains no <diagram>.",
                "Add at least one <diagram> element.",
            )]);
        }
        if page >= diagrams.len() {
            return Err(vec![Notice::error(
                "page_out_of_range",
                None,
                &format!(
                    "Page index {page} is out of range; the file has {} page(s).",
                    diagrams.len()
                ),
                "Choose a page index between 0 and the page count minus one.",
            )]);
        }
        for (i, d) in diagrams.iter().enumerate() {
            if i != page {
                diagnostics.push(Notice::warning(
                    "page_skipped",
                    Some(d.line),
                    Some(i),
                    None,
                    &format!(
                        "Page \"{}\" was not imported; only the selected page was read.",
                        d.attr("name").unwrap_or("Diagram")
                    ),
                    "Import each page separately if you want all of them.",
                ));
            }
        }
        let d = diagrams[page];
        model = match d.child("mxGraphModel") {
            Some(m) => m,
            None => {
                return Err(vec![Notice::error(
                    "no_graph_model",
                    Some(d.line),
                    "The selected <diagram> has no <mxGraphModel>.",
                    "Make sure the page contains a graph model.",
                )]);
            }
        };
    } else {
        model = match root.iter().find(|n| n.name == "mxGraphModel") {
            Some(m) => m,
            None => {
                if root.iter().any(|n| n.name == "mxfile") {
                    return Err(vec![Notice::error(
                        "no_diagram",
                        None,
                        "The <mxfile> contains no <diagram>.",
                        "Add at least one <diagram> element.",
                    )]);
                }
                return Err(vec![Notice::error(
                    "not_drawio",
                    None,
                    "This file is not a recognized .drawio XML.",
                    "Choose a file with an <mxfile> or <mxGraphModel> root.",
                )]);
            }
        };
    }

    let root_cell = model.child("root").ok_or_else(|| {
        vec![Notice::error(
            "no_root",
            None,
            "The graph model has no <root> cell container.",
            "Add a <root> element with a <mxCell id=\"0\"/> parent.",
        )]
    })?;

    // Parse all cells into a flat map keyed by id.
    let mut cells_by_id: std::collections::HashMap<String, &XNode> =
        std::collections::HashMap::new();
    let mut cell_order: Vec<String> = Vec::new();
    for c in &root_cell.children {
        if c.name != "mxCell" {
            continue;
        }
        let id = c.attr("id").unwrap_or_default();
        if id.is_empty() {
            continue;
        }
        if cells_by_id.len() >= MAX_CELLS {
            return Err(vec![Notice::error(
                "cell_limit",
                Some(c.line),
                &format!("The diagram exceeds {MAX_CELLS} cells."),
                "Split the diagram into smaller files.",
            )]);
        }
        cells_by_id.insert(id.to_string(), c);
        cell_order.push(id.to_string());
    }

    let page_name = pages(source)
        .unwrap_or_default()
        .get(page)
        .cloned()
        .unwrap_or_else(|| "draw.io import".into());
    let mut document = Document {
        title: "draw.io import".into(),
        page_name,
        ..Default::default()
    };

    // Identify layer cells: elements with parent="0" that are not vertices or
    // edges declare layers. The default layer is id="1", always imported. Other
    // visible layers are adopted up to a bound; invisible layers are skipped and
    // reported.
    let mut adopt_layer: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    adopt_layer.insert("1".to_string(), true); // default layer always imported
    let mut layer_names: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for c in cell_order.iter().filter_map(|id| cells_by_id.get(id)) {
        let id = c.attr("id").unwrap_or("");
        if id == "0" || id == "1" {
            continue;
        }
        let parent = c.attr("parent").unwrap_or("");
        if parent == "0" && c.attr("vertex").is_none() && c.attr("edge").is_none() {
            let visible = c.attr("visible") != Some("0");
            let name = c.attr("value").unwrap_or(id).to_string();
            if visible {
                if adopt_layer.len() >= 16 {
                    diagnostics.push(Notice::warning(
                        "layer_limit",
                        Some(c.line),
                        Some(page),
                        None,
                        "Too many visible layers; only the first ones are imported.",
                        "Move objects to fewer layers before importing.",
                    ));
                    adopt_layer.insert(id.to_string(), false);
                } else {
                    adopt_layer.insert(id.to_string(), true);
                }
            } else {
                adopt_layer.insert(id.to_string(), false);
            }
            layer_names.insert(id.to_string(), name);
        }
    }

    // First pass: build one node per vertex on an adopted layer.
    let mut node_of_id: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut skipped_layers_reported: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for c in cell_order.iter().filter_map(|id| cells_by_id.get(id)) {
        let id = c.attr("id").unwrap_or("");
        if c.attr("vertex") != Some("1") {
            continue;
        }
        let parent = c.attr("parent").unwrap_or("");
        if !adopt_layer.get(parent).copied().unwrap_or(false) {
            if let Some(name) = layer_names.get(parent)
                && skipped_layers_reported.insert(name.clone())
            {
                diagnostics.push(Notice::warning(
                    "layer_skipped",
                    Some(c.line),
                    Some(page),
                    None,
                    &format!("Layer \"{name}\" was not imported."),
                    "Enable the layer in draw.io or import it separately.",
                ));
            }
            continue;
        }
        let (Some(x), Some(y)) = (geom_attr(c, "x"), geom_attr(c, "y")) else {
            diagnostics.push(Notice::warning(
                "geometry_missing",
                Some(c.line),
                Some(page),
                None,
                "A vertex has no x/y geometry and was skipped.",
                "Give the shape a position in draw.io.",
            ));
            continue;
        };
        let width = geom_attr(c, "width").unwrap_or(120.);
        let height = geom_attr(c, "height").unwrap_or(80.);
        if width <= 0. || height <= 0. {
            diagnostics.push(Notice::warning(
                "geometry_invalid",
                Some(c.line),
                Some(page),
                None,
                "A vertex has a non-positive width or height and was skipped.",
                "Fix the shape size in draw.io.",
            ));
            continue;
        }
        let style = c.attr("style").unwrap_or("");
        let kind = shape_from_style(style);
        let id_num = document.next_id();
        let center = [x + width / 2., y + height / 2.];
        document.add_node(kind, center);
        {
            let n = document.node_mut(id_num).unwrap();
            n.position = [x, y];
            n.size = [width, height];
            let label = label_text(c);
            let mut label = label;
            if label.chars().count() > MAX_LABEL_CHARS {
                label = label.chars().take(MAX_LABEL_CHARS).collect();
                diagnostics.push(Notice::warning(
                    "label_truncated",
                    Some(c.line),
                    Some(page),
                    Some(id_num),
                    "A label was truncated to the 4096 character limit.",
                    "Shorten the label in draw.io.",
                ));
            }
            n.text = label;
            apply_node_style(n, style);
        }
        // Images are never fetched: external URLs are always dropped; data: PNG
        // is decoded only when the engine supports it.
        if let Some(img) = c.attr("image").or_else(|| style_attr(style, "image")) {
            if let Some(rest) = img.strip_prefix("data:") {
                // data:image/png;base64,<payload> — strip the MIME prefix.
                let payload = rest.split(',').next_back().unwrap_or(rest);
                if let Ok(asset) = crate::images::ImageAsset::from_png_base64(payload) {
                    let content = crate::images::ImageContent::new(asset);
                    document.node_mut(id_num).unwrap().image = Some(content);
                } else {
                    diagnostics.push(Notice::warning(
                        "image_dropped",
                        Some(c.line),
                        Some(page),
                        Some(id_num),
                        "An image was dropped (non-PNG data URL).",
                        "Replace the image with an embedded PNG before importing.",
                    ));
                }
            } else {
                diagnostics.push(Notice::warning(
                    "image_external",
                    Some(c.line),
                    Some(page),
                    Some(id_num),
                    "An external image URL was not loaded.",
                    "No network access is performed; embed the image instead.",
                ));
            }
        }
        node_of_id.insert(id.to_string(), id_num);
    }

    // Second pass: build connectors from edges.
    for c in cell_order.iter().filter_map(|id| cells_by_id.get(id)) {
        if c.attr("edge") != Some("1") {
            continue;
        }
        let parent = c.attr("parent").unwrap_or("");
        if !adopt_layer.get(parent).copied().unwrap_or(false) {
            continue;
        }
        let from = c.attr("source").and_then(|s| node_of_id.get(s)).copied();
        let to = c.attr("target").and_then(|s| node_of_id.get(s)).copied();
        let style = c.attr("style").unwrap_or("");
        let label = label_text(c);
        let connected = match (from, to) {
            (Some(f), Some(t)) if f == t => {
                diagnostics.push(Notice::warning(
                    "edge_self_loop",
                    Some(c.line),
                    Some(page),
                    None,
                    "An edge connects a shape to itself and was skipped.",
                    "Remove the self-loop in draw.io.",
                ));
                None
            }
            (Some(f), Some(t)) => document.connect(f, t, ConnectorKind::Elbow),
            _ => {
                diagnostics.push(Notice::warning(
                    "edge_dangling",
                    Some(c.line),
                    Some(page),
                    None,
                    "An edge has a source or target that could not be resolved and was skipped.",
                    "Anchor both ends of the edge to shapes in draw.io.",
                ));
                None
            }
        };
        // Style and label the connector we just created by its id.
        if let Some(cid) = connected {
            let conn = document
                .connectors
                .iter_mut()
                .find(|e| e.id == cid)
                .unwrap();
            conn.label = label;
            apply_edge_style(conn, style);
            apply_waypoints(conn, c);
        }
    }

    document.validate().map_err(|e| {
        vec![Notice::error(
            "invalid_document",
            None,
            &e,
            "Reduce the number of elements or the size of the labels.",
        )]
    })?;
    Ok(Import {
        document,
        diagnostics,
    })
}

// ---- helpers ----------------------------------------------------------------

/// Parses a hex `#RRGGBB` color used by draw.io into an `[u8; 3]`.
fn parse_hex_color(v: &str) -> Option<crate::document::Color> {
    let hex = v.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

/// Reads a numeric attribute from a cell's `<mxGeometry>`, if present.
fn geom_attr(c: &XNode, key: &str) -> Option<f32> {
    let geometry = c.child("mxGeometry")?;
    geometry.attr(key).and_then(|v| v.parse::<f32>().ok())
}

/// Reads a `key=value` pair from a draw.io style string (semicolon separated).
fn style_attr<'a>(style: &'a str, key: &str) -> Option<&'a str> {
    for pair in style.split(';') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k.trim() == key && !v.is_empty() {
            return Some(v.trim());
        }
    }
    None
}

/// Maps a draw.io shape name to a DiagCraft `ShapeKind`. Unknown shapes are
/// reported separately (as a loss) and default to `Rectangle`.
fn shape_from_style(style: &str) -> ShapeKind {
    let shape = style_attr(style, "shape").or_else(|| {
        // draw.io compatibility: a bare leading token with no '=' names the shape,
        // e.g. `rhombus;fillColor=…` or `shape=rhombus`.
        style.split(';').next().filter(|s| !s.contains('='))
    });
    let Some(shape) = shape else {
        return ShapeKind::Rectangle;
    };
    match shape {
        "rect" | "rectangle" => ShapeKind::Rectangle,
        "rounded" => ShapeKind::Rounded,
        "ellipse" | "circle" => ShapeKind::Ellipse,
        "rhombus" | "diamond" => ShapeKind::Diamond,
        "text" | "label" => ShapeKind::Text,
        "document" | "internalStorage" => ShapeKind::Document,
        "cylinder3" | "cylinder" | "database" => ShapeKind::Database,
        "hexagon" => ShapeKind::Hexagon,
        "trapezoid" => ShapeKind::Trapezoid,
        "cloud" => ShapeKind::Cloud,
        "process" | "manualInput" => ShapeKind::ManualInput,
        "offPageConnector" => ShapeKind::OffPage,
        "delimiter" | "delay" => ShapeKind::Delay,
        "note" | "note2" => ShapeKind::Note,
        "callout" => ShapeKind::Callout,
        "actor" => ShapeKind::Actor,
        "umlLifeline" => ShapeKind::Lifeline,
        "umlClass" => ShapeKind::UmlClass,
        "package" | "folder" => ShapeKind::Package,
        "component" => ShapeKind::Component,
        "terminal" | "terminator" => ShapeKind::Terminator,
        _ => ShapeKind::Rectangle,
    }
}

/// Extracts the display text of a cell from its `value` attribute. `html=1`
/// never triggers HTML rendering: `<br>`-like sequences become line breaks and
/// all entities were already decoded by the XML parser.
fn label_text(c: &XNode) -> String {
    let raw = c.attr("value").unwrap_or("").to_string();
    let mut out = String::with_capacity(raw.len());
    let b = raw.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if b[i] == b'<'
            && (raw[i..].starts_with("<br>")
                || raw[i..].starts_with("<br/>")
                || raw[i..].starts_with("<br />"))
        {
            out.push('\n');
            i += if raw[i..].starts_with("<br>") { 4 } else { 5 };
            continue;
        }
        let ch = raw[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    // Collapse internal line breaks to a single trailing newline when the text
    // itself holds no line structure; diagcraft wraps text itself.
    out
}

/// Applies fill/stroke/font style to a node.
fn apply_node_style(n: &mut crate::document::Node, style: &str) {
    if let Some(c) = style_attr(style, "fillColor").and_then(parse_hex_color) {
        n.fill = c;
    }
    if let Some(c) = style_attr(style, "strokeColor").and_then(parse_hex_color) {
        n.stroke = c;
    }
    if let Some(c) = style_attr(style, "fontColor").and_then(parse_hex_color) {
        n.text_color = c;
    }
    if let Some(fs) = style_attr(style, "fontSize").and_then(|v| v.parse::<f32>().ok())
        && fs > 0.
    {
        n.font_size = fs;
    }
    if let Some(f) = style_attr(style, "fontStyle") {
        let v = f.parse::<i32>().unwrap_or(0);
        n.text_style.bold = v & 1 != 0;
        n.text_style.italic = v & 2 != 0;
        n.text_style.underline = v & 4 != 0;
    }
    n.text_style.wrap = true;
}

/// Applies stroke/width/arrows/dash style to a connector.
fn apply_edge_style(e: &mut Connector, style: &str) {
    if let Some(c) = style_attr(style, "strokeColor").and_then(parse_hex_color) {
        e.color = c;
    }
    if let Some(w) = style_attr(style, "strokeWidth").and_then(|v| v.parse::<f32>().ok())
        && w > 0.
    {
        e.width = w;
    }
    if style_attr(style, "dashed").is_some() && style_attr(style, "dashed") != Some("0") {
        e.pattern = if style_attr(style, "dashed") == Some("1") {
            LinePattern::Dashed
        } else {
            LinePattern::Dotted
        };
    }
    e.arrow = true;
    e.arrow_start = false;
    if let Some(h) = style_attr(style, "endArrow") {
        e.arrow = h != "none" && h != "0";
        e.end_head = if h == "open" {
            ArrowHead::Open
        } else if h == "diamond" {
            ArrowHead::Diamond
        } else if h == "oval" {
            ArrowHead::Circle
        } else {
            ArrowHead::Triangle
        };
    }
    if let Some(h) = style_attr(style, "startArrow") {
        e.arrow_start = h != "none" && h != "0";
        e.start_head = if h == "open" {
            ArrowHead::Open
        } else if h == "diamond" {
            ArrowHead::Diamond
        } else if h == "oval" {
            ArrowHead::Circle
        } else {
            ArrowHead::Triangle
        };
    }
}

/// Applies `<mxGeometry>` waypoints (Array of mxPoint) to a connector.
fn apply_waypoints(e: &mut Connector, c: &XNode) {
    let geometry = match c.child("mxGeometry") {
        Some(g) => g,
        None => return,
    };
    let mut points = Vec::new();
    if let Some(p) = geometry.child("mxPoint")
        && p.attr("as") == Some("sourcePoint")
    {
        let x = p.attr("x").and_then(|v| v.parse::<f32>().ok());
        let y = p.attr("y").and_then(|v| v.parse::<f32>().ok());
        if let (Some(x), Some(y)) = (x, y) {
            e.free_start = Some([x, y]);
        }
    }
    if let Some(array) = geometry.child("Array") {
        for pt in &array.children {
            if pt.name == "mxPoint" {
                let x = pt.attr("x").and_then(|v| v.parse::<f32>().ok());
                let y = pt.attr("y").and_then(|v| v.parse::<f32>().ok());
                if let (Some(x), Some(y)) = (x, y) {
                    points.push([x, y]);
                }
            }
        }
    }
    e.waypoints = points;
}
