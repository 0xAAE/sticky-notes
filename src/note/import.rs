use std::{io::Cursor, path::Path};

use anyhow::Context;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Deserializer, de::Error};
use uuid::Uuid;

pub async fn try_import_indicator_stickynotes<P: AsRef<Path> + std::fmt::Debug>(
    data_file: P,
) -> anyhow::Result<Vec<Note>> {
    let content = tokio::fs::read(&data_file).await?;
    let parsed: Notes = serde_json::from_reader(Cursor::new(content))
        .with_context(|| "while parsing json".to_string())?;
    Ok(parsed.notes)
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Note {
    pub uuid: Uuid,
    pub body: String,
    #[serde(deserialize_with = "deserialize_from_str")]
    pub last_modified: DateTime<Utc>,
    pub properties: NoteProperties,
}

fn deserialize_from_str<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let tmp = NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S").map_err(D::Error::custom)?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(tmp, Utc))
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct NoteProperties {
    pub position: Vec<usize>,
    pub size: Vec<usize>,
    pub locked: bool,
}

#[derive(serde::Deserialize)]
pub struct Notes {
    pub notes: Vec<Note>,
}
