//! Spacecrafts database
use anyhow::Result;
use std::env;
use std::fs::File;
use std::io::{Error as IoError, ErrorKind};
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub type APID = u16;
pub type SCID = u16;
pub type VCID = u16;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename = "APID")]
pub struct APIDInfo {
    pub apid: APID,
    pub description: String,
    pub sensor: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename = "VCID")]
pub struct VCIDInfo {
    pub vcid: VCID,
    pub description: String,
    pub apids: Vec<APIDInfo>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RSConfig {
    pub interleave: u8,
    pub virtual_fill_length: usize,
    pub num_correctable: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct PnConfig {}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FramingConfig {
    /// Length of the frame, not including any rs parity bytes. See ``cadu_len``.
    pub length: usize,
    pub insert_zone_length: usize,
    pub trailer_length: usize,
    pub pseudo_noise: Option<PnConfig>,
    pub reed_solomon: Option<RSConfig>,
}

impl FramingConfig {
    /// Length of a cadu payload or codeblock for this config
    #[must_use]
    pub fn codeblock_len(&self) -> usize {
        match &self.reed_solomon {
            Some(rs) => self.length + 2 * rs.num_correctable as usize * rs.interleave as usize,
            None => self.length,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Spacecraft {
    pub scid: SCID,
    pub name: String,
    pub aliases: Vec<String>,
    pub catalog_number: u32,
    pub framing_config: FramingConfig,
    pub vcids: Vec<VCIDInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DB {
    #[serde(skip)]
    path: String,
    version: String,
    git_sha: String,
    generated: String,
    spacecrafts: Vec<Spacecraft>,
}

impl DB {
    fn locate_db() -> Result<String> {
        let project = "spacecraftsdb";
        let filename = format!("{project}.json");

        // cwd
        let mut paths = vec![PathBuf::from_str(&filename)?];

        // XDG_DATA_HOME
        if let Ok(s) = env::var("XDG_DATA_HOME") {
            let path = PathBuf::new().join(s).join(project).join(&filename);
            paths.push(path);
        }

        // HOME
        if let Some(homedir) = dirs::home_dir() {
            paths.push(homedir.join(format!(".{filename}")));
        }

        let mut tried = vec![];
        for path in &paths {
            if path.exists() && path.is_file() {
                return Ok(path.to_str().unwrap().to_string());
            }
            tried.push(format!("{path:?}"));
        }

        Err(anyhow::Error::new(IoError::new(
            ErrorKind::NotFound,
            "Unable to locate spacecraftsdb at any of the following locations: ".to_string()
                + &tried.join(", "),
        )))
    }

    /// Creates a new ``DB`` by searching for the spacecrafts db file in `./spacecraftsdb.json`,
    /// `$XDG_DATA_HOME/spacecraftsdb/spacecraftsdb.json`, then `$HOME/.spacecraftsdb.json`.
    ///
    /// # Errors
    /// If the database json file cannot be found
    pub fn new() -> Result<Self> {
        DB::with_path(&DB::locate_db()?)
    }

    /// Creates a new ``DB`` using the provided path to spacecrafts json file.
    ///
    /// # Errors
    /// If the format of the provided file does not match the expected database format.
    pub fn with_path(path: &str) -> Result<Self> {
        let f = File::open(path)?;
        let mut db: DB = serde_json::from_reader(f)?;
        db.path = path.to_string();
        Ok(db)
    }

    /// Find the spacecraft with the provided identifier. Returns `None` if the spacecraft
    /// cannot be found.
    #[must_use]
    pub fn find(&self, scid: SCID) -> Option<Spacecraft> {
        self.spacecrafts.iter().find(|sc| sc.scid == scid).cloned()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_with_path() {
        let path: PathBuf = [
            &std::env::var("CARGO_MANIFEST_DIR").unwrap(),
            "tests",
            "fixtures",
            "spacecraftsdb.json",
        ]
        .iter()
        .collect();
        let db = DB::with_path(path.to_str().unwrap()).expect("failed to load database");

        assert_eq!(path.to_string_lossy(), db.path);
        assert_eq!(db.spacecrafts.len(), 1);
        assert_eq!(db.spacecrafts[0].scid, 157);
    }
}
