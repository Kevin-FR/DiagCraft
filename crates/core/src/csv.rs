//! Original, bounded CSV → diagram conversion. The supported subset is defined
//! in docs/csv.md before implementation. CSV is treated as local data: no network,
//! no formula or macro execution, no external resource loading.
use crate::document::{ConnectorKind, Document, ShapeKind};
use serde::Serialize;
use std::collections::BTreeMap;

pub const MAX_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_ROWS: usize = 1000;
pub const MAX_COLUMNS: usize = 64;
pub const MAX_CELL_CHARS: usize = 4096;

/// A parsed CSV table: header + rows, each record one `Vec` of cell values.
pub type Records = Vec<Vec<String>>;

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
    fn error(
        code: &str,
        line: Option<usize>,
        column: Option<usize>,
        message: &str,
        hint: &str,
    ) -> Self {
        Self {
            code: code.into(),
            severity: "error".into(),
            line,
            column,
            page: None,
            id: None,
            message: message.into(),
            hint: hint.into(),
        }
    }
    fn warning(code: &str, line: Option<usize>, message: &str, hint: &str) -> Self {
        Self {
            code: code.into(),
            severity: "warning".into(),
            line,
            column: None,
            page: None,
            id: None,
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

/// Column selections for the conversion.
#[derive(Clone, Debug, Default)]
pub struct CsvOptions {
    /// Header name whose value becomes each shape's label. Defaults to the first column.
    pub label: Option<String>,
    /// Header name whose values are stable references for relations. Optional unless
    /// relations are requested, in which case it is required.
    pub identity: Option<String>,
    /// Header names whose values reference other rows (identities, `|`-separated).
    pub connect: Vec<String>,
}

/// Converts bounded CSV source into a single editable page. Returns `Err` when a
/// fatal limit or structure error is found; otherwise the diagnostics contain any
/// non-blocking losses. The source is immutable.
pub fn import(source: &str, options: &CsvOptions) -> Result<Import, Vec<Notice>> {
    let (header, rows) = parse_table(source)?;
    let label_index = resolve_index(&header, options.label.as_deref())?;
    let identity_index = match &options.identity {
        Some(name) => Some(resolve_index(&header, Some(name))?),
        None => None,
    };
    if !options.connect.is_empty() && identity_index.is_none() {
        return Err(vec![Notice::error(
            "identity_required",
            None,
            None,
            "Relations need an identity column.",
            "Choose an identity column that uniquely identifies each row, or remove the relation columns.",
        )]);
    }
    let mut connect_indexes = Vec::new();
    for name in &options.connect {
        connect_indexes.push(resolve_index(&header, Some(name))?);
    }

    // Index rows by their key (identity value, or trimmed label when no identity
    // column is set) so relations resolve deterministically by value, not by
    // physical row number.
    let mut by_key: BTreeMap<String, usize> = BTreeMap::new();
    let mut key_of: Vec<String> = vec![String::new(); rows.len()];
    let mut diagnostics = Vec::new();
    let mut node_ids: Vec<Option<u64>> = vec![None; rows.len()];
    for (row_i, row) in rows.iter().enumerate() {
        if is_blank(row) {
            diagnostics.push(Notice::warning(
                "blank_row",
                Some(line_of(row_i)),
                "Empty row ignored; no shape was created.",
                "Remove blank lines or leave them empty if intentional.",
            ));
            continue;
        }
        let value = match identity_index {
            Some(ix) => row.get(ix).cloned().unwrap_or_default(),
            None => row.get(label_index).cloned().unwrap_or_default(),
        };
        let key = value.trim().to_string();
        if !key.is_empty() {
            if let Some(&existing) = by_key.get(&key) {
                return Err(vec![Notice::error(
                    "duplicate_identity",
                    Some(line_of(row_i)),
                    None,
                    &format!(
                        "Duplicate key \"{key}\" on rows {} and {}.",
                        existing + 1,
                        row_i + 1
                    ),
                    "Give each row a unique value in the identity (or label) column.",
                )]);
            }
            by_key.insert(key.clone(), row_i);
            key_of[row_i] = key;
        }
    }

    if rows.iter().all(|r| is_blank(r)) {
        return Err(vec![Notice::error(
            "empty_data",
            None,
            None,
            "No data rows remain after removing blank lines.",
            "Add at least one non-empty data row below the header.",
        )]);
    }

    // Build one node per non-blank data row, placed on a deterministic bounded grid.
    let mut document = Document {
        title: "CSV import".into(),
        ..Default::default()
    };
    let cell_w = 240.0f32;
    let cell_h = 120.0f32;
    let gap_x = 40.0f32;
    let gap_y = 40.0f32;
    let per_row = 6usize;
    let mut kept: Vec<usize> = Vec::new();
    for (row_i, row) in rows.iter().enumerate() {
        if is_blank(row) {
            continue;
        }
        let col = kept.len() % per_row;
        let r = kept.len() / per_row;
        let position = [
            gap_x + col as f32 * (cell_w + gap_x) + cell_w / 2.,
            gap_y + r as f32 * (cell_h + gap_y) + cell_h / 2.,
        ];
        let id = document.add_node(ShapeKind::Rounded, position);
        let n = document.node_mut(id).unwrap();
        n.text = row.get(label_index).cloned().unwrap_or_default();
        n.text_style.wrap = true;
        node_ids[row_i] = Some(id);
        kept.push(row_i);
    }

    // Relations reference the key of another row's shape.
    if let Some(ix) = identity_index {
        for &row_i in &kept {
            let from_id = node_ids[row_i].unwrap();
            for &connect_ix in &connect_indexes {
                let cell = rows[row_i].get(connect_ix).cloned().unwrap_or_default();
                for target in cell.split('|').map(str::trim).filter(|s| !s.is_empty()) {
                    match by_key.get(target).copied().and_then(|ti| node_ids[ti]) {
                        Some(to_id) => {
                            document.connect(from_id, to_id, ConnectorKind::Elbow);
                        }
                        None => {
                            diagnostics.push(Notice::warning(
                                "missing_relation",
                                Some(line_of(row_i)),
                                &format!(
                                    "Relation \"{target}\" has no matching row (column {}).",
                                    header[ix]
                                ),
                                "Make sure the value matches the identity of an existing row.",
                            ));
                        }
                    }
                }
            }
        }
    }
    let columns = per_row.min(kept.len());
    let rows_count = kept.len().div_ceil(per_row);
    document.page_size = [
        gap_x * 2. + columns as f32 * (cell_w + gap_x),
        gap_y * 2. + rows_count as f32 * (cell_h + gap_y),
    ];
    document.validate().map_err(|e| {
        vec![Notice::error(
            "invalid_document",
            None,
            None,
            &e,
            "Reduce the number of rows or the size of the labels.",
        )]
    })?;
    diagnostics.push(Notice::warning(
        "layout_changed",
        None,
        "DiagCraft generates an editable grid layout from the CSV data.",
        "Review node positions and connector routes before applying; no layout is preserved from the CSV.",
    ));
    Ok(Import {
        document,
        diagnostics,
    })
}

/// Returns the column names (header) of a CSV source, using the same bounded
/// parser as `import`. Used by the interface to let the user choose columns.
pub fn headers(source: &str) -> Result<Vec<String>, Vec<Notice>> {
    let (header, _) = parse_table(source)?;
    Ok(header)
}

/// Shared bounded parse: source size, delimiter detection, record parsing,
/// header extraction and header validation.
fn parse_table(source: &str) -> Result<(Vec<String>, Records), Vec<Notice>> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(vec![Notice::error(
            "source_limit",
            None,
            None,
            "CSV source exceeds 256 KiB.",
            "Split the data into smaller CSV files or reduce the number of rows.",
        )]);
    }
    let delim = detect_delimiter(source);
    let records = parse_records(source, delim).map_err(|(line, msg)| {
        vec![Notice::error(
            "csv_parse",
            Some(line),
            None,
            &msg,
            "Fix the quoted fields or newlines in the CSV data.",
        )]
    })?;
    let (header, rows) = split_header(records)?;
    validate_headers(&header)?;
    Ok((header, rows))
}

fn line_of(row_i: usize) -> usize {
    row_i + 2
}

fn is_blank(row: &[String]) -> bool {
    row.iter().all(|c| c.trim().is_empty())
}

fn detect_delimiter(source: &str) -> char {
    let first = source.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let commas = first.matches(',').count();
    let semis = first.matches(';').count();
    if semis > commas { ';' } else { ',' }
}

/// Splits parsed records into a header and data rows. A leading completely blank
/// record (before the header) is tolerated and dropped.
fn split_header(records: Records) -> Result<(Vec<String>, Records), Vec<Notice>> {
    let mut records = records;
    while !records.is_empty() && is_blank(&records[0]) {
        records.remove(0);
    }
    if records.is_empty() {
        return Err(vec![Notice::error(
            "empty_header",
            None,
            None,
            "The CSV file has no header row.",
            "Add a first row with column names.",
        )]);
    }
    let header = records.remove(0);
    Ok((header, records))
}

/// Headers must be non-empty, unique and within the column budget.
fn validate_headers(header: &[String]) -> Result<(), Vec<Notice>> {
    if header.is_empty() {
        return Err(vec![Notice::error(
            "empty_header",
            None,
            None,
            "The CSV file has no columns.",
            "Add at least one column name to the header row.",
        )]);
    }
    if header.len() > MAX_COLUMNS {
        return Err(vec![Notice::error(
            "column_limit",
            Some(1),
            None,
            &format!("CSV header exceeds {MAX_COLUMNS} columns."),
            "Reduce the number of columns to fit the supported subset.",
        )]);
    }
    let mut seen = BTreeMap::new();
    for (i, name) in header.iter().enumerate() {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(vec![Notice::error(
                "empty_header",
                Some(1),
                Some(i + 1),
                "A column has no name.",
                "Fill in or remove the empty column name in the header row.",
            )]);
        }
        if let Some(&first) = seen.get(trimmed) {
            return Err(vec![Notice::error(
                "duplicate_header",
                Some(1),
                Some(i + 1),
                &format!(
                    "Duplicate column name \"{trimmed}\" (also column {}).",
                    first + 1
                ),
                "Rename one of the duplicate columns.",
            )]);
        }
        seen.insert(trimmed.to_string(), i);
    }
    Ok(())
}

fn resolve_index(header: &[String], name: Option<&str>) -> Result<usize, Vec<Notice>> {
    match name {
        None => Ok(0),
        Some(req) => header
            .iter()
            .position(|h| h.trim() == req.trim())
            .ok_or_else(|| {
                vec![Notice::error(
                    "column_not_found",
                    None,
                    None,
                    &format!("Column \"{req}\" was not found in the header."),
                    &format!(
                        "Choose one of the available columns: {}.",
                        header.join(", ")
                    ),
                )]
            }),
    }
}

/// RFC 4180 subset: quoted fields with `""` escapes and embedded newlines; a
/// trailing CR is trimmed. Bounded to MAX_ROWS records and MAX_COLUMNS fields,
/// MAX_CELL_CHARS per field.
fn parse_records(source: &str, delim: char) -> Result<Records, (usize, String)> {
    let mut records: Records = Vec::new();
    let mut field = String::new();
    let mut record: Vec<String> = Vec::new();
    let mut in_quotes = false;
    let mut line = 1usize;
    let mut record_start_line = 1usize;
    let mut chars = source.chars().peekable();

    // Flush the current field into the current record, enforcing the column budget
    // and cell-size budget. Returns an error carrying the offending line.
    macro_rules! end_field {
        () => {{
            if record.len() >= MAX_COLUMNS {
                return Err((line, format!("CSV record exceeds {MAX_COLUMNS} columns.")));
            }
            if field.chars().count() > MAX_CELL_CHARS {
                return Err((
                    line,
                    format!("CSV cell exceeds {MAX_CELL_CHARS} characters."),
                ));
            }
            record.push(std::mem::take(&mut field));
        }};
    }
    // Flush the current record (field + record) once a record boundary is hit.
    macro_rules! end_record {
        () => {{
            end_field!();
            if records.len() >= MAX_ROWS {
                return Err((
                    record_start_line,
                    format!("CSV file exceeds {MAX_ROWS} data rows."),
                ));
            }
            records.push(std::mem::take(&mut record));
        }};
    }

    while let Some(c) = chars.next() {
        if in_quotes {
            match c {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                '\n' => {
                    field.push('\n');
                    line += 1;
                }
                _ => field.push(c),
            }
        } else {
            match c {
                '"' => {
                    if field.is_empty() {
                        in_quotes = true;
                    } else {
                        field.push('"');
                    }
                }
                '\r' => {
                    // CRLF: ignore CR, the following LF (if any) ends the record.
                }
                '\n' => {
                    end_record!();
                    line += 1;
                    record_start_line = line;
                }
                c if c == delim => {
                    end_field!();
                }
                _ => field.push(c),
            }
        }
    }
    // End of input: close any open quote error, then flush the last record unless
    // it is empty and the file ended with a newline.
    if in_quotes {
        return Err((record_start_line, "Unterminated quoted field.".into()));
    }
    if !field.is_empty() || !record.is_empty() {
        end_record!();
    }
    Ok(records)
}
