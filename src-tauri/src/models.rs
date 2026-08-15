use serde::{Deserialize, Serialize};

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
    pub paths: Vec<String>,
    pub mode: OutputMode,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanResult {
    pub source_path: String,
    pub output_path: Option<String>,
    pub backup_path: Option<String>,
    pub removed: Vec<Finding>,
    pub success: bool,
    pub error: Option<String>,
}
