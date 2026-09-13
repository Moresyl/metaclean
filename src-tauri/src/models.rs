use serde::{
    de::{Error as DeError, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::fmt;

use crate::{MAX_BATCH_FILES, MAX_BATCH_PATH_BYTES, MAX_PATH_BYTES};

#[derive(Debug, Clone)]
pub struct BoundedString<const MAX_BYTES: usize>(String);

impl<const MAX_BYTES: usize> BoundedString<MAX_BYTES> {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<const MAX_BYTES: usize> Default for BoundedString<MAX_BYTES> {
    fn default() -> Self {
        Self(String::new())
    }
}

struct BoundedStringVisitor<const MAX_BYTES: usize>;

impl<'de, const MAX_BYTES: usize> Visitor<'de> for BoundedStringVisitor<MAX_BYTES> {
    type Value = BoundedString<MAX_BYTES>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "a string no longer than {MAX_BYTES} bytes")
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_str(value)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if value.len() > MAX_BYTES {
            return Err(E::custom(format!("string exceeds {MAX_BYTES} bytes")));
        }
        Ok(BoundedString(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if value.len() > MAX_BYTES {
            return Err(E::custom(format!("string exceeds {MAX_BYTES} bytes")));
        }
        Ok(BoundedString(value))
    }
}

impl<'de, const MAX_BYTES: usize> Deserialize<'de> for BoundedString<MAX_BYTES> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(BoundedStringVisitor::<MAX_BYTES>)
    }
}

pub type BoundedBatchId = BoundedString<128>;
pub type BoundedUpdateVersion = BoundedString<128>;

#[derive(Debug, Clone)]
pub struct BoundedPaths(Vec<String>);

impl BoundedPaths {
    pub fn into_inner(self) -> Vec<String> {
        self.0
    }
}

struct BoundedPath(String);

struct PathVisitor;

impl<'de> Visitor<'de> for PathVisitor {
    type Value = BoundedPath;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a non-empty path no longer than 32 KiB")
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        self.visit_str(value)
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if value.is_empty() || value.len() > MAX_PATH_BYTES {
            return Err(E::custom(format!(
                "path must be non-empty and at most {MAX_PATH_BYTES} bytes"
            )));
        }
        Ok(BoundedPath(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: DeError,
    {
        if value.is_empty() || value.len() > MAX_PATH_BYTES {
            return Err(E::custom(format!(
                "path must be non-empty and at most {MAX_PATH_BYTES} bytes"
            )));
        }
        Ok(BoundedPath(value))
    }
}

impl<'de> Deserialize<'de> for BoundedPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Ask the deserializer for a borrowed/string view first. This lets
        // serde reject oversized path text before allocating an owned copy.
        deserializer.deserialize_str(PathVisitor)
    }
}

struct PathsVisitor;

impl<'de> Visitor<'de> for PathsVisitor {
    type Value = BoundedPaths;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded array of paths")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut paths = Vec::new();
        let mut total_bytes = 0usize;
        while let Some(path) = sequence.next_element::<BoundedPath>()? {
            if paths.len() >= MAX_BATCH_FILES {
                return Err(A::Error::custom(format!(
                    "at most {MAX_BATCH_FILES} paths are allowed"
                )));
            }
            total_bytes = total_bytes
                .checked_add(path.0.len())
                .ok_or_else(|| A::Error::custom("path payload is too large"))?;
            if total_bytes > MAX_BATCH_PATH_BYTES {
                return Err(A::Error::custom(format!(
                    "path payload exceeds {MAX_BATCH_PATH_BYTES} bytes"
                )));
            }
            paths.push(path.0);
        }
        Ok(BoundedPaths(paths))
    }
}

impl<'de> Deserialize<'de> for BoundedPaths {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(PathsVisitor)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub category: String,
    pub label: String,
    pub count: usize,
    pub severity: FindingSeverity,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Privacy,
    Provenance,
    Informational,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub path: String,
    pub name: String,
    pub format: String,
    pub size: u64,
    pub supported: bool,
    pub findings: Vec<Finding>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    Copy,
    Replace,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanRequest {
    pub paths: BoundedPaths,
    #[serde(default)]
    pub batch_id: BoundedBatchId,
    pub mode: OutputMode,
    #[serde(default = "default_true")]
    pub preserve_timestamps: bool,
    #[serde(default = "default_true")]
    pub preserve_orientation: bool,
    #[serde(default = "default_true")]
    pub preserve_color_profile: bool,
    #[serde(default)]
    pub remove_extended_attributes: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub source_path: String,
    pub output_path: Option<String>,
    pub backup_path: Option<String>,
    pub source_size: Option<u64>,
    pub output_size: Option<u64>,
    pub removed: Vec<Finding>,
    pub success: bool,
    pub error: Option<String>,
}
