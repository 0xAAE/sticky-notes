use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Deserializer, Serializer, de::Error};
use serde_json_fmt::JsonSyntaxError;
use std::{collections::HashMap, io::Cursor, path::Path};
use thiserror::Error;
use uuid::Uuid;

const COMMA_FORMAT: &str = ", ";

#[derive(Debug, Error)]
pub enum IndicatorStickyNotesError {
    // Failed reading source file
    #[error("Failed reading file: {0}")]
    Io(#[from] std::io::Error),
    // Failed parsing JSON-content
    #[error("Failed parsing collection of notes: {0}")]
    Parse(serde_json::Error),
    // Failed formatting JSON-content: no indentation
    #[error("Failed setting indentation to None: {0}")]
    FormatIndent(JsonSyntaxError),
    // Failed formatting JSON-content: spacebar after comma
    #[error("Failed setting comma to {COMMA_FORMAT}: {0}")]
    FormatComma(JsonSyntaxError),
    // Failed parsing JSON-content
    #[error("Failed serializing collection of notes: {0}")]
    Json(serde_json::Error),
}

pub async fn try_import_indicator_stickynotes<P: AsRef<Path> + std::fmt::Debug>(
    data_file: P,
) -> Result<NotesDatabase, IndicatorStickyNotesError> {
    let content = tokio::fs::read(data_file).await?;
    NotesDatabase::try_import(&content)
}

pub async fn try_export_indicator_stickynotes<P: AsRef<Path> + std::fmt::Debug>(
    data_file: P,
    data_base: NotesDatabase,
) -> Result<(), IndicatorStickyNotesError> {
    let content = data_base.try_export()?;
    tokio::fs::write(data_file, content)
        .await
        .map_err(IndicatorStickyNotesError::Io)
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub struct Note {
    pub uuid: Uuid,
    pub body: String,
    #[serde(
        deserialize_with = "deserialize_from_str",
        serialize_with = "serialize_to_str"
    )]
    pub last_modified: DateTime<Local>,
    pub properties: NoteProperties,
    pub cat: Uuid,
}

const IMPORT_DATETIME_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";

fn deserialize_from_str<'de, D>(deserializer: D) -> Result<DateTime<Local>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let tmp =
        NaiveDateTime::parse_from_str(&s, IMPORT_DATETIME_FORMAT).map_err(D::Error::custom)?;
    Ok(Local
        .from_local_datetime(&tmp)
        .single()
        // if failed for some reason use local time
        .unwrap_or_else(Local::now))
}

fn serialize_to_str<S>(value: &DateTime<Local>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.format(IMPORT_DATETIME_FORMAT).to_string())
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub struct NoteProperties {
    pub position: Vec<usize>,
    pub size: Vec<usize>,
    pub locked: bool,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub struct GlobalProperties {
    pub all_visible: bool,
    pub default_cat: Uuid,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq)]
pub struct CategoryProperties {
    pub name: String,
    pub bgcolor_hsv: Vec<f32>,
    pub font: String,
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
pub struct NotesDatabase {
    pub notes: Vec<Note>,
    pub properties: GlobalProperties,
    pub categories: HashMap<Uuid, CategoryProperties>,
}

impl NotesDatabase {
    pub fn try_get_default_category(&self) -> Option<&CategoryProperties> {
        self.categories.get(&self.properties.default_cat)
    }

    fn try_import(content: &[u8]) -> Result<Self, IndicatorStickyNotesError> {
        serde_json::from_reader(Cursor::new(content)).map_err(IndicatorStickyNotesError::Parse)
    }

    /// produce output file as similar to imported source as possible:
    /// * no '\n'
    /// * no extra indentations
    /// * preserve notes order
    /// * using local time zone
    /// * (!) but allow to violate categories order
    fn try_export(&self) -> Result<Vec<u8>, IndicatorStickyNotesError> {
        serde_json_fmt::JsonFormat::pretty()
            .ascii(true)
            .indent::<String>(None)
            .map_err(IndicatorStickyNotesError::FormatComma)?
            .comma(COMMA_FORMAT)
            .map_err(IndicatorStickyNotesError::FormatComma)?
            .format_to_string(self)
            .map_err(IndicatorStickyNotesError::Json)
            .map(|s| s.as_bytes().to_vec())
    }
}

#[test]
fn import_end_export() {
    const INPUT_FILE: &str = "test_data/indicator-stickynotes";

    // read source file into buffer
    let buf = std::fs::read(INPUT_FILE).expect(format!("reading input file {INPUT_FILE}").as_str());
    assert!(!buf.is_empty());

    // parse buffer content, then test expected values
    let parsed = NotesDatabase::try_import(&buf).expect("import must succeed");
    assert_eq!(parsed.notes.len(), 7);
    assert!(parsed.properties.all_visible);
    assert!(!parsed.properties.default_cat.is_nil());
    assert_eq!(parsed.categories.len(), 8);
    let default_category = parsed.try_get_default_category();
    assert!(default_category.is_some());
    let default_category = default_category.unwrap();
    assert_eq!(default_category.name.as_str(), "Green");
    assert_eq!(default_category.bgcolor_hsv.len(), 3);
    assert_eq!(default_category.font.as_str(), "Fira Sans 14");

    // serialize parsed into string, then compare to the expected text
    let export = parsed.try_export().expect("export must succeed");
    assert!(!export.is_empty());
    // do not compare with source as categories order may differ

    // parse from the export again and compare to previously parsed
    let parsed_again = NotesDatabase::try_import(&export).expect("import from export must succeed");
    assert_eq!(parsed, parsed_again);
}
