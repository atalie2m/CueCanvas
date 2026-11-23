use std::collections::{BTreeMap, BTreeSet};

use cuecanvas_model::{DataTable, EntityType, Origin, Person, Session, ShowDefinition, Sponsor};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataImportRequest {
    pub entity_type: EntityType,
    pub source_name: String,
    pub format: DelimitedFormat,
    pub content: String,
    pub mappings: Vec<ColumnMapping>,
    #[serde(default)]
    pub replace_existing: bool,
    pub mode: Option<ImportMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DelimitedFormat {
    Csv,
    Tsv,
}

impl DelimitedFormat {
    fn separator(&self) -> char {
        match self {
            Self::Csv => ',',
            Self::Tsv => '\t',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
    Append,
    Replace,
    Merge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataImportReport {
    pub entity_type: EntityType,
    pub source_name: String,
    pub imported_rows: usize,
    pub rejected_rows: usize,
    pub duplicate_rows: usize,
    pub conflicted_rows: usize,
    pub warnings: Vec<String>,
    pub issues: Vec<ImportValidationIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImportValidationIssue {
    pub row_index: usize,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedImport {
    rows: Vec<BTreeMap<String, String>>,
    report: DataImportReport,
}

#[derive(Debug, Error)]
pub enum DataImportError {
    #[error("import content is empty")]
    EmptyContent,
    #[error("import has no header row")]
    MissingHeader,
    #[error("CSV/TSV parse error: {0}")]
    Parse(String),
    #[error("mapping references missing source column: {0}")]
    MissingSourceColumn(String),
    #[error("unsupported target field for {entity_type:?}: {field}")]
    UnsupportedTargetField {
        entity_type: EntityType,
        field: String,
    },
    #[error("required target field is not mapped for {entity_type:?}: {field}")]
    MissingRequiredMapping {
        entity_type: EntityType,
        field: String,
    },
}

pub fn apply_import(
    show: &mut ShowDefinition,
    request: DataImportRequest,
) -> Result<DataImportReport, DataImportError> {
    let parsed = build_import(&request)?;
    let mode = apply_mode(&request);
    let origin = Origin::Import {
        source: request.source_name.clone(),
    };

    let mut report = parsed.report;
    let rows = parsed.rows;

    let table_rows = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|(key, value)| (key.clone(), json!(value)))
                .collect::<BTreeMap<_, _>>()
        })
        .collect::<Vec<_>>();
    let data_table = DataTable {
        id: data_table_id(&request.entity_type),
        entity_type: request.entity_type.clone(),
        rows: table_rows,
        origin: origin.clone(),
        extensions: BTreeMap::new(),
    };

    match request.entity_type {
        EntityType::People => {
            let people = rows
                .iter()
                .map(|row| person_from_row(row, &origin))
                .collect::<Vec<_>>();
            apply_people(show, people, &mode, &mut report);
        }
        EntityType::Sessions => {
            let sessions = rows
                .iter()
                .map(|row| session_from_row(row, &origin))
                .collect::<Vec<_>>();
            apply_sessions(show, sessions, &mode, &mut report);
        }
        EntityType::Sponsors => {
            let sponsors = rows
                .iter()
                .map(|row| sponsor_from_row(row, &origin))
                .collect::<Vec<_>>();
            apply_sponsors(show, sponsors, &mode, &mut report);
        }
    }

    upsert_data_table(show, data_table, matches!(mode, ImportMode::Replace));
    Ok(report)
}

pub fn parse_import(request: &DataImportRequest) -> Result<DataImportReport, DataImportError> {
    Ok(build_import(request)?.report)
}

fn build_import(request: &DataImportRequest) -> Result<ParsedImport, DataImportError> {
    let table = parse_delimited(&request.content, request.format.separator())?;
    let header = table.first().ok_or(DataImportError::MissingHeader)?;
    if header.is_empty() {
        return Err(DataImportError::MissingHeader);
    }

    validate_mappings(request, header)?;

    let mut warnings = Vec::new();
    let mut issues = Vec::new();
    let mut rows = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let required = required_field(&request.entity_type);

    for (row_index, raw_row) in table.iter().skip(1).enumerate() {
        if raw_row.iter().all(|value| value.trim().is_empty()) {
            continue;
        }

        let mut mapped = BTreeMap::new();
        for mapping in &request.mappings {
            let source_index = header
                .iter()
                .position(|column| column == &mapping.source)
                .expect("mappings are validated before row conversion");
            let value = raw_row
                .get(source_index)
                .map(|value| value.trim().to_string())
                .unwrap_or_default();
            mapped.insert(mapping.target.clone(), value);
        }

        if mapped
            .get(required)
            .is_none_or(|value| value.trim().is_empty())
        {
            issues.push(ImportValidationIssue {
                row_index: row_index + 2,
                field: required.to_string(),
                message: "Required field is empty".to_string(),
            });
            continue;
        }

        ensure_id(&request.entity_type, &mut mapped, &mut seen_ids);
        rows.push(mapped);
    }

    if rows.is_empty() && issues.is_empty() {
        warnings.push("Import contained no data rows".to_string());
    }

    Ok(ParsedImport {
        report: DataImportReport {
            entity_type: request.entity_type.clone(),
            source_name: request.source_name.clone(),
            imported_rows: rows.len(),
            rejected_rows: issues.len(),
            duplicate_rows: 0,
            conflicted_rows: 0,
            warnings,
            issues,
        },
        rows,
    })
}

fn apply_mode(request: &DataImportRequest) -> ImportMode {
    request.mode.clone().unwrap_or_else(|| {
        if request.replace_existing {
            ImportMode::Replace
        } else {
            ImportMode::Append
        }
    })
}

fn parse_delimited(content: &str, separator: char) -> Result<Vec<Vec<String>>, DataImportError> {
    if content.trim().is_empty() {
        return Err(DataImportError::EmptyContent);
    }

    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut chars = content.chars().peekable();
    let mut quoted = false;

    while let Some(ch) = chars.next() {
        if quoted {
            match ch {
                '"' if chars.peek() == Some(&'"') => {
                    cell.push('"');
                    let _ = chars.next();
                }
                '"' => quoted = false,
                _ => cell.push(ch),
            }
            continue;
        }

        match ch {
            '"' if cell.is_empty() => quoted = true,
            value if value == separator => {
                row.push(cell);
                cell = String::new();
            }
            '\n' => {
                row.push(cell);
                cell = String::new();
                rows.push(row);
                row = Vec::new();
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    let _ = chars.next();
                }
                row.push(cell);
                cell = String::new();
                rows.push(row);
                row = Vec::new();
            }
            _ => cell.push(ch),
        }
    }

    if quoted {
        return Err(DataImportError::Parse(
            "unterminated quoted field".to_string(),
        ));
    }

    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }

    Ok(rows)
}

fn validate_mappings(
    request: &DataImportRequest,
    header: &[String],
) -> Result<(), DataImportError> {
    let header_set = header.iter().collect::<BTreeSet<_>>();
    let mut targets = BTreeSet::new();

    for mapping in &request.mappings {
        if !header_set.contains(&mapping.source) {
            return Err(DataImportError::MissingSourceColumn(mapping.source.clone()));
        }
        if !allowed_fields(&request.entity_type).contains(&mapping.target.as_str()) {
            return Err(DataImportError::UnsupportedTargetField {
                entity_type: request.entity_type.clone(),
                field: mapping.target.clone(),
            });
        }
        targets.insert(mapping.target.as_str());
    }

    let required = required_field(&request.entity_type);
    if !targets.contains(required) {
        return Err(DataImportError::MissingRequiredMapping {
            entity_type: request.entity_type.clone(),
            field: required.to_string(),
        });
    }

    Ok(())
}

fn allowed_fields(entity_type: &EntityType) -> &'static [&'static str] {
    match entity_type {
        EntityType::People => &["id", "displayName", "role", "organization", "photo"],
        EntityType::Sessions => &["id", "title", "track", "startTime", "speakerRefs"],
        EntityType::Sponsors => &["id", "name", "logo", "tier"],
    }
}

fn required_field(entity_type: &EntityType) -> &'static str {
    match entity_type {
        EntityType::People => "displayName",
        EntityType::Sessions => "title",
        EntityType::Sponsors => "name",
    }
}

fn ensure_id(
    entity_type: &EntityType,
    row: &mut BTreeMap<String, String>,
    seen_ids: &mut BTreeSet<String>,
) {
    let current = row.get("id").map(|value| value.trim()).unwrap_or_default();
    let base = if current.is_empty() {
        let required_value = row
            .get(required_field(entity_type))
            .map(String::as_str)
            .unwrap_or("entity");
        format!("{}-{}", id_prefix(entity_type), slug(required_value))
    } else {
        slug(current)
    };

    let mut candidate = base.clone();
    let mut suffix = 2;
    while seen_ids.contains(&candidate) {
        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
    seen_ids.insert(candidate.clone());
    row.insert("id".to_string(), candidate);
}

fn id_prefix(entity_type: &EntityType) -> &'static str {
    match entity_type {
        EntityType::People => "person",
        EntityType::Sessions => "session",
        EntityType::Sponsors => "sponsor",
    }
}

fn slug(value: &str) -> String {
    let mut output = String::new();
    let mut previous_dash = false;

    for ch in value.to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }

    let output = output.trim_matches('-').to_string();
    if output.is_empty() {
        "entity".to_string()
    } else {
        output
    }
}

fn optional(row: &BTreeMap<String, String>, field: &str) -> Option<String> {
    row.get(field)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn person_from_row(row: &BTreeMap<String, String>, origin: &Origin) -> Person {
    Person {
        id: row["id"].clone(),
        display_name: row["displayName"].clone(),
        role: optional(row, "role"),
        organization: optional(row, "organization"),
        photo: optional(row, "photo"),
        origin: origin.clone(),
        extensions: BTreeMap::new(),
    }
}

fn session_from_row(row: &BTreeMap<String, String>, origin: &Origin) -> Session {
    Session {
        id: row["id"].clone(),
        title: row["title"].clone(),
        track: optional(row, "track"),
        start_time: optional(row, "startTime"),
        speaker_refs: optional(row, "speakerRefs")
            .map(|value| {
                value
                    .split(';')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        origin: origin.clone(),
        extensions: BTreeMap::new(),
    }
}

fn sponsor_from_row(row: &BTreeMap<String, String>, origin: &Origin) -> Sponsor {
    Sponsor {
        id: row["id"].clone(),
        name: row["name"].clone(),
        logo: optional(row, "logo"),
        tier: optional(row, "tier"),
        origin: origin.clone(),
        extensions: BTreeMap::new(),
    }
}

fn data_table_id(entity_type: &EntityType) -> String {
    match entity_type {
        EntityType::People => "typed-people".to_string(),
        EntityType::Sessions => "typed-sessions".to_string(),
        EntityType::Sponsors => "typed-sponsors".to_string(),
    }
}

fn upsert_data_table(show: &mut ShowDefinition, table: DataTable, replace_existing: bool) {
    if let Some(existing) = show
        .data_tables
        .iter_mut()
        .find(|existing| existing.entity_type == table.entity_type)
    {
        existing.origin = table.origin;
        if replace_existing {
            existing.rows = table.rows;
        } else {
            existing.rows.extend(table.rows);
        }
    } else {
        show.data_tables.push(table);
    }
}

fn apply_people(
    show: &mut ShowDefinition,
    incoming: Vec<Person>,
    mode: &ImportMode,
    report: &mut DataImportReport,
) {
    match mode {
        ImportMode::Replace => show.typed_entities.people = incoming,
        ImportMode::Append => {
            let existing = show
                .typed_entities
                .people
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<BTreeSet<_>>();
            let mut accepted = Vec::new();
            for entity in incoming {
                if existing.contains(entity.id.as_str()) {
                    report.duplicate_rows += 1;
                    report.rejected_rows += 1;
                    report.issues.push(ImportValidationIssue {
                        row_index: 0,
                        field: "id".to_string(),
                        message: format!("Duplicate person id '{}'", entity.id),
                    });
                } else {
                    accepted.push(entity);
                }
            }
            report.imported_rows = accepted.len();
            show.typed_entities.people.extend(accepted);
        }
        ImportMode::Merge => {
            let mut appended = 0;
            for entity in incoming {
                if let Some(existing) = show
                    .typed_entities
                    .people
                    .iter_mut()
                    .find(|existing| existing.id == entity.id)
                {
                    if existing != &entity {
                        report.conflicted_rows += 1;
                    }
                    *existing = entity;
                } else {
                    appended += 1;
                    show.typed_entities.people.push(entity);
                }
            }
            report.imported_rows = appended + report.conflicted_rows;
        }
    }
}

fn apply_sessions(
    show: &mut ShowDefinition,
    incoming: Vec<Session>,
    mode: &ImportMode,
    report: &mut DataImportReport,
) {
    match mode {
        ImportMode::Replace => show.typed_entities.sessions = incoming,
        ImportMode::Append => {
            let existing = show
                .typed_entities
                .sessions
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<BTreeSet<_>>();
            let mut accepted = Vec::new();
            for entity in incoming {
                if existing.contains(entity.id.as_str()) {
                    report.duplicate_rows += 1;
                    report.rejected_rows += 1;
                    report.issues.push(ImportValidationIssue {
                        row_index: 0,
                        field: "id".to_string(),
                        message: format!("Duplicate session id '{}'", entity.id),
                    });
                } else {
                    accepted.push(entity);
                }
            }
            report.imported_rows = accepted.len();
            show.typed_entities.sessions.extend(accepted);
        }
        ImportMode::Merge => {
            let mut appended = 0;
            for entity in incoming {
                if let Some(existing) = show
                    .typed_entities
                    .sessions
                    .iter_mut()
                    .find(|existing| existing.id == entity.id)
                {
                    if existing != &entity {
                        report.conflicted_rows += 1;
                    }
                    *existing = entity;
                } else {
                    appended += 1;
                    show.typed_entities.sessions.push(entity);
                }
            }
            report.imported_rows = appended + report.conflicted_rows;
        }
    }
}

fn apply_sponsors(
    show: &mut ShowDefinition,
    incoming: Vec<Sponsor>,
    mode: &ImportMode,
    report: &mut DataImportReport,
) {
    match mode {
        ImportMode::Replace => show.typed_entities.sponsors = incoming,
        ImportMode::Append => {
            let existing = show
                .typed_entities
                .sponsors
                .iter()
                .map(|entity| entity.id.as_str())
                .collect::<BTreeSet<_>>();
            let mut accepted = Vec::new();
            for entity in incoming {
                if existing.contains(entity.id.as_str()) {
                    report.duplicate_rows += 1;
                    report.rejected_rows += 1;
                    report.issues.push(ImportValidationIssue {
                        row_index: 0,
                        field: "id".to_string(),
                        message: format!("Duplicate sponsor id '{}'", entity.id),
                    });
                } else {
                    accepted.push(entity);
                }
            }
            report.imported_rows = accepted.len();
            show.typed_entities.sponsors.extend(accepted);
        }
        ImportMode::Merge => {
            let mut appended = 0;
            for entity in incoming {
                if let Some(existing) = show
                    .typed_entities
                    .sponsors
                    .iter_mut()
                    .find(|existing| existing.id == entity.id)
                {
                    if existing != &entity {
                        report.conflicted_rows += 1;
                    }
                    *existing = entity;
                } else {
                    appended += 1;
                    show.typed_entities.sponsors.push(entity);
                }
            }
            report.imported_rows = appended + report.conflicted_rows;
        }
    }
}

#[cfg(test)]
mod tests {
    use cuecanvas_engine::demo_project;

    use super::*;

    #[test]
    fn imports_people_csv_with_quotes_and_reflects_data_table() {
        let mut project = demo_project();
        let show = project.show_definitions.first_mut().unwrap();
        let report = apply_import(
            show,
            DataImportRequest {
                entity_type: EntityType::People,
                source_name: "speakers.csv".to_string(),
                format: DelimitedFormat::Csv,
                content: "name,role,org\n\"Ada, Lovelace\",Keynote,Analytical Engines\nGrace Hopper,Admiral,Navy\n"
                    .to_string(),
                mappings: vec![
                    mapping("name", "displayName"),
                    mapping("role", "role"),
                    mapping("org", "organization"),
                ],
                replace_existing: true,
                mode: None,
            },
        )
        .unwrap();

        assert_eq!(report.imported_rows, 2);
        assert_eq!(report.rejected_rows, 0);
        assert_eq!(show.typed_entities.people[0].id, "person-ada-lovelace");
        assert_eq!(show.typed_entities.people[0].display_name, "Ada, Lovelace");
        assert_eq!(
            show.data_tables
                .iter()
                .find(|table| table.entity_type == EntityType::People)
                .unwrap()
                .rows[0]["displayName"],
            json!("Ada, Lovelace")
        );
    }

    #[test]
    fn rejects_rows_missing_required_session_title() {
        let request = DataImportRequest {
            entity_type: EntityType::Sessions,
            source_name: "sessions.tsv".to_string(),
            format: DelimitedFormat::Tsv,
            content: "title\ttrack\n\tMain\nClosing\tMain\n".to_string(),
            mappings: vec![mapping("title", "title"), mapping("track", "track")],
            replace_existing: true,
            mode: None,
        };

        let report = parse_import(&request).unwrap();

        assert_eq!(report.imported_rows, 1);
        assert_eq!(report.rejected_rows, 1);
        assert_eq!(report.issues[0].row_index, 2);
        assert_eq!(report.issues[0].field, "title");
    }

    #[test]
    fn imports_sponsors_tsv_with_import_origin() {
        let mut project = demo_project();
        let show = project.show_definitions.first_mut().unwrap();

        apply_import(
            show,
            DataImportRequest {
                entity_type: EntityType::Sponsors,
                source_name: "sponsors.tsv".to_string(),
                format: DelimitedFormat::Tsv,
                content: "name\ttier\tlogo\nACME\tGold\tasset-acme\n".to_string(),
                mappings: vec![
                    mapping("name", "name"),
                    mapping("tier", "tier"),
                    mapping("logo", "logo"),
                ],
                replace_existing: true,
                mode: None,
            },
        )
        .unwrap();

        assert_eq!(show.typed_entities.sponsors[0].id, "sponsor-acme");
        assert_eq!(
            show.typed_entities.sponsors[0].tier.as_deref(),
            Some("Gold")
        );
        assert_eq!(
            show.typed_entities.sponsors[0].origin,
            Origin::Import {
                source: "sponsors.tsv".to_string()
            }
        );
    }

    #[test]
    fn append_rejects_existing_duplicate_ids_and_merge_updates_conflicts() {
        let mut project = demo_project();
        let show = project.show_definitions.first_mut().unwrap();

        let append_report = apply_import(
            show,
            DataImportRequest {
                entity_type: EntityType::People,
                source_name: "people.csv".to_string(),
                format: DelimitedFormat::Csv,
                content: "id,name,role\nperson-cat,Cat Again,CEO\n".to_string(),
                mappings: vec![
                    mapping("id", "id"),
                    mapping("name", "displayName"),
                    mapping("role", "role"),
                ],
                replace_existing: false,
                mode: Some(ImportMode::Append),
            },
        )
        .unwrap();
        assert_eq!(append_report.duplicate_rows, 1);
        assert_eq!(append_report.rejected_rows, 1);
        assert_eq!(show.typed_entities.people[0].display_name, "Cat");

        let merge_report = apply_import(
            show,
            DataImportRequest {
                entity_type: EntityType::People,
                source_name: "people.csv".to_string(),
                format: DelimitedFormat::Csv,
                content: "id,name,role\nperson-cat,Cat Again,CEO\n".to_string(),
                mappings: vec![
                    mapping("id", "id"),
                    mapping("name", "displayName"),
                    mapping("role", "role"),
                ],
                replace_existing: false,
                mode: Some(ImportMode::Merge),
            },
        )
        .unwrap();
        assert_eq!(merge_report.conflicted_rows, 1);
        assert_eq!(show.typed_entities.people[0].display_name, "Cat Again");
    }

    #[test]
    fn rejects_unknown_target_fields() {
        let error = parse_import(&DataImportRequest {
            entity_type: EntityType::People,
            source_name: "people.csv".to_string(),
            format: DelimitedFormat::Csv,
            content: "name,color\nAda,blue\n".to_string(),
            mappings: vec![
                mapping("name", "displayName"),
                mapping("color", "favoriteColor"),
            ],
            replace_existing: true,
            mode: None,
        })
        .unwrap_err();

        assert!(matches!(
            error,
            DataImportError::UnsupportedTargetField { field, .. } if field == "favoriteColor"
        ));
    }

    fn mapping(source: &str, target: &str) -> ColumnMapping {
        ColumnMapping {
            source: source.to_string(),
            target: target.to_string(),
        }
    }
}
