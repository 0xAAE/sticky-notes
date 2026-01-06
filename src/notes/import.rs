use std::{collections::HashMap, io::Cursor, path::Path};

use anyhow::Context;
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, de::Error};
use uuid::Uuid;

pub async fn try_import_indicator_stickynotes<P: AsRef<Path> + std::fmt::Debug>(
    data_file: P,
) -> anyhow::Result<NotesDatabase> {
    let content = tokio::fs::read(&data_file).await?;
    let parsed: NotesDatabase = serde_json::from_reader(Cursor::new(content))
        .with_context(|| "while parsing json".to_string())?;
    Ok(parsed)
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Note {
    pub uuid: Uuid,
    pub body: String,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub last_modified: DateTime<Utc>,
    pub properties: NoteProperties,
    pub cat: Uuid,
}

fn deserialize_from_str<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let tmp = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").map_err(D::Error::custom)?;
    // try to assume as local time, otherwise assume as UTC time
    Ok(Local.from_local_datetime(&tmp).single().map_or_else(
        || DateTime::<Utc>::from_naive_utc_and_offset(tmp, Utc),
        |local| local.with_timezone(&Utc),
    ))
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct NoteProperties {
    pub position: Vec<usize>,
    pub size: Vec<usize>,
    pub locked: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct GlobalProperties {
    pub all_visible: bool,
    pub default_cat: Uuid,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct CategoryProperties {
    pub name: String,
    pub bgcolor_hsv: Vec<f32>,
    pub font: String,
}

#[derive(serde::Deserialize)]
pub struct NotesDatabase {
    pub notes: Vec<Note>,
    pub properties: GlobalProperties,
    pub categories: HashMap<Uuid, CategoryProperties>,
}

impl NotesDatabase {
    pub fn try_get_default_category(&self) -> Option<&CategoryProperties> {
        self.categories.get(&self.properties.default_cat)
    }
}

#[test]
fn import_local_sample() -> anyhow::Result<()> {
    const INPUT_FILE: &str = "test_data/indicator-stickynotes";
    let buf = std::fs::read(INPUT_FILE).expect(format!("reading input file {INPUT_FILE}").as_str());
    assert!(!buf.is_empty());
    let parsed: NotesDatabase = serde_json::from_reader(Cursor::new(buf))
        .with_context(|| "while parsing json".to_string())?;
    assert_eq!(parsed.notes.len(), 6);
    assert!(parsed.properties.all_visible);
    assert!(!parsed.properties.default_cat.is_nil());
    assert_eq!(parsed.categories.len(), 8);
    let default_category = parsed.try_get_default_category();
    assert!(default_category.is_some());
    let default_category = default_category.unwrap();
    assert_eq!(default_category.name.as_str(), "Green");
    assert_eq!(default_category.bgcolor_hsv.len(), 3);
    assert_eq!(default_category.font.as_str(), "Fira Sans 14");
    Ok(())
}
