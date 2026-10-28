//! Additive diagnostic envelope for local agent workflows. Never stores patches or pixels.
use super::{ObjectRef, Operation};
use serde::Serialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct Failure {
    pub diagnostic: Box<Diagnostic>,
}
fn bounded(s: &str) -> String {
    let mut out: String = s.chars().take(1000).collect();
    if out.len() < s.len() {
        out.push('…');
    }
    out
}
fn alias(s: &str) -> Value {
    if s.len() <= 100 {
        json!(s)
    } else {
        json!("<alias trop long>")
    }
}
impl Failure {
    pub fn new(code: &str, message: impl AsRef<str>, hint: &str) -> Self {
        Self {
            diagnostic: Box::new(Diagnostic {
                code: code.into(),
                severity: "error".into(),
                message: bounded(message.as_ref()),
                hint: hint.into(),
                operation: None,
                operation_name: None,
                page: None,
                target: None,
                id: None,
                line: None,
                column: None,
            }),
        }
    }
    pub(crate) fn arguments(error: &serde_json::Error) -> Self {
        Self::new(
            "invalid_arguments",
            schema_message(error),
            "Utilisez exactement les arguments et types de tools/list pour cet outil.",
        )
    }
    pub fn to_value(&self) -> Value {
        json!({"error":self.to_string(),"diagnostic":self.diagnostic})
    }
    pub fn location(mut self, line: usize, column: usize) -> Self {
        if line > 0 {
            self.diagnostic.line = Some(line);
            self.diagnostic.column = Some(column.max(1));
        }
        self
    }
    pub(super) fn operation(
        mut self,
        index: usize,
        op: &Operation,
        aliases: &BTreeMap<String, (usize, u64)>,
    ) -> Self {
        let (name, page, target) = context(op);
        self.diagnostic.operation = Some(index);
        self.diagnostic.operation_name = Some(name.into());
        self.diagnostic.page = page;
        if let Some(target) = target {
            self.diagnostic.id = target.as_u64().or_else(|| {
                target.as_str().and_then(|a| {
                    aliases
                        .get(a)
                        .filter(|(p, _)| Some(*p) == page)
                        .map(|(_, id)| *id)
                })
            });
            self.diagnostic.target = Some(target);
        }
        self
    }
    pub(super) fn raw_operation(mut self, index: usize, op: &Value) -> Self {
        self.diagnostic.operation = Some(index);
        self.diagnostic.operation_name = op["op"]
            .as_str()
            .filter(|s| s.len() <= 100 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
            .map(str::to_owned);
        self.diagnostic.page = op["page"].as_u64().and_then(|p| usize::try_from(p).ok());
        if let Some(v) = op
            .get("target")
            .or_else(|| op.get("layer"))
            .or_else(|| op.get("alias"))
        {
            self.diagnostic.target = match v {
                Value::String(s) => Some(alias(s)),
                Value::Number(_) => Some(v.clone()),
                _ => None,
            };
            self.diagnostic.id = v.as_u64();
        }
        self
    }
}
impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = &self.diagnostic;
        if let Some(i) = d.operation {
            write!(f, "Opération {}", i + 1)?;
            if let Some(name) = &d.operation_name {
                write!(f, " ({name})")?;
            }
            write!(f, " : ")?;
        }
        if let Some(page) = d.page {
            write!(f, "page {page}")?;
            if let Some(target) = &d.target {
                write!(f, " · cible {target}")?;
            }
            write!(f, " — ")?;
        }
        write!(f, "{}", d.message)?;
        if let Some(line) = d.line {
            write!(f, " (ligne {line}, colonne {})", d.column.unwrap_or(1))?;
        }
        write!(f, " Conseil : {}", d.hint)
    }
}
impl std::error::Error for Failure {}
fn reference(r: &ObjectRef) -> Value {
    match r {
        ObjectRef::Id(id) => json!(id),
        ObjectRef::Alias(s) => alias(s),
    }
}
fn context(op: &Operation) -> (&'static str, Option<usize>, Option<Value>) {
    use Operation::*;
    let (name, page, target) = match op {
        AddNode { page, alias, .. } => ("add_node", Some(*page), Some(json!(alias))),
        UpdateNode { page, target, .. } => ("update_node", Some(*page), Some(reference(target))),
        AddImage { page, alias, .. } => ("add_image", Some(*page), Some(json!(alias))),
        UpdateImage { page, target, .. } => ("update_image", Some(*page), Some(reference(target))),
        AddConnector { page, alias, .. } => ("add_connector", Some(*page), Some(json!(alias))),
        UpdateConnector { page, target, .. } => {
            ("update_connector", Some(*page), Some(reference(target)))
        }
        AddChart { page, alias, .. } => ("add_chart", Some(*page), Some(json!(alias))),
        UpdateChart { page, target, .. } => ("update_chart", Some(*page), Some(reference(target))),
        AddDrawing { page, alias, .. } => ("add_drawing", Some(*page), Some(json!(alias))),
        UpdateDrawing { page, target, .. } => {
            ("update_drawing", Some(*page), Some(reference(target)))
        }
        Delete { page, target } => ("delete", Some(*page), Some(reference(target))),
        GroupObjects { page, targets } => (
            "group_objects",
            Some(*page),
            Some(Value::Array(
                targets.iter().take(20).map(reference).collect(),
            )),
        ),
        UngroupObjects { page, targets } => (
            "ungroup_objects",
            Some(*page),
            Some(Value::Array(
                targets.iter().take(20).map(reference).collect(),
            )),
        ),
        SetObjectsLocked { page, targets, .. } => (
            "set_objects_locked",
            Some(*page),
            Some(Value::Array(
                targets.iter().take(20).map(reference).collect(),
            )),
        ),
        AddLayer { page, alias, .. } => ("add_layer", Some(*page), Some(json!(alias))),
        UpdateLayer { page, layer, .. } => ("update_layer", Some(*page), Some(reference(layer))),
        MoveLayer { page, layer, .. } => ("move_layer", Some(*page), Some(reference(layer))),
        MoveObjectsToLayer { page, targets, .. } => (
            "move_objects_to_layer",
            Some(*page),
            Some(Value::Array(
                targets.iter().take(20).map(reference).collect(),
            )),
        ),
        DeleteLayer { page, layer } => ("delete_layer", Some(*page), Some(reference(layer))),
        SetTitle { .. } => ("set_title", None, None),
        AddPage { .. } => ("add_page", None, None),
        UpdatePage { page, .. } => ("update_page", Some(*page), None),
        MovePage { from, .. } => ("move_page", Some(*from), None),
        DeletePage { page } => ("delete_page", Some(*page), None),
        FitPage { page } => ("fit_page", Some(*page), None),
    };
    let target = target.map(|v| match v {
        Value::String(s) => alias(&s),
        other => other,
    });
    (name, page, target)
}
pub(super) fn page(op: &Operation) -> Option<usize> {
    context(op).1
}
/// Domain model errors stay strings for compatibility; only known categories are promoted.
pub(super) fn domain(message: String) -> Failure {
    let (code, hint) = if message.contains("verrouill") || message.contains("protég") {
        (
            "object_locked",
            "Déverrouillez les objets ou leur calque dans une opération distincte avant de les modifier.",
        )
    } else if message.starts_with("Alias inconnu")
        || message.starts_with("Alias de calque introuvable")
    {
        (
            "alias_not_found",
            "Utilisez un alias créé plus tôt dans ce lot, sur la même page, ou un ID issu de draw_inspect.",
        )
    } else if message.contains("introuvable") || message.contains("absente") {
        (
            "object_not_found",
            "Inspectez la page et utilisez l’ID local d’un objet existant du type attendu.",
        )
    } else if message.contains("Géométrie")
        || message.contains("géométrie")
        || message.contains("Dimensions")
    {
        (
            "invalid_geometry",
            "Utilisez des coordonnées finies (±100000 px), des blocs de 24 à 20000 px et des pages de 100 à 100000 px.",
        )
    } else if message.contains("Alias vide") {
        (
            "invalid_alias",
            "Choisissez un alias unique de 1 à 100 octets, puis réutilisez-le uniquement sur sa page.",
        )
    } else if message.contains("maximum") || message.contains("trop") || message.contains("dépasse")
    {
        (
            "resource_limit",
            "Réduisez la taille, le nombre d’objets ou les données d’images ; consultez les budgets dans draw_catalog.",
        )
    } else if message.contains("extrémité") {
        (
            "invalid_endpoint",
            "Chaque extrémité exige node ou point, jamais les deux ; un point libre utilise le port auto.",
        )
    } else {
        (
            "invalid_operation",
            "Consultez draw_catalog pour les champs, types et limites ; corrigez cette opération puis réessayez le lot complet.",
        )
    };
    let safe_message = if message.starts_with("Alias inconnu") {
        "Alias introuvable sur cette page."
    } else {
        &message
    };
    Failure::new(code, safe_message, hint)
}
/// Serde's data errors may contain arbitrary values; only safe field/variant names are surfaced.
pub(super) fn schema_message(error: &serde_json::Error) -> String {
    let text = error.to_string();
    for (prefix, label) in [
        ("unknown field `", "Champ inconnu"),
        ("missing field `", "Champ requis absent"),
        ("unknown variant `", "Type inconnu"),
    ] {
        if let Some(rest) = text.strip_prefix(prefix) {
            let field = rest.split('`').next().unwrap_or("");
            if field.len() <= 100
                && field
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return format!("{label} : {field}.");
            }
        }
    }
    "Valeur incompatible avec le schéma du patch.".into()
}
/// Valid JSON is scanned only to locate the beginning of each operation, not to reparse values.
pub(super) fn operation_offsets(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    let mut operations = false;
    let mut pending = false;
    let mut out = vec![];
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                let start = i;
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        break;
                    }
                    i += 1;
                }
                if depth == 1 && &text[start..i.min(bytes.len() - 1) + 1] == "\"operations\"" {
                    let mut j = i + 1;
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                    if bytes.get(j) == Some(&b':') {
                        pending = true;
                    }
                }
            }
            b'[' => {
                depth += 1;
                if pending && depth == 2 {
                    operations = true;
                    pending = false;
                }
            }
            b'{' => {
                if operations && depth == 2 {
                    out.push(i);
                }
                depth += 1;
            }
            b']' => {
                if operations && depth == 2 {
                    operations = false;
                }
                depth = depth.saturating_sub(1);
            }
            b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        i += 1;
    }
    out
}
pub(super) fn location(text: &str, offset: usize) -> (usize, usize) {
    let prefix = &text.as_bytes()[..offset.min(text.len())];
    let line = prefix.iter().filter(|b| **b == b'\n').count() + 1;
    let column = prefix
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or(prefix.len() + 1, |p| prefix.len() - p);
    (line, column)
}
