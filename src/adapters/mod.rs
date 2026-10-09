pub mod cargo;
pub(crate) mod limits;
use crate::report::normalize::ArtifactRef;
use serde::{Deserialize, Serialize};
#[derive(schemars::JsonSchema, Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutorProfile {
    pub tool: String,
    pub version: String,
    pub protocol: String,
    pub target: String,
    pub features: Vec<String>,
    pub parameters: String,
    pub environment: String,
}
#[derive(schemars::JsonSchema, Clone, Debug)]
pub struct RawArtifactSet {
    pub attempt_id: String,
    pub inventory: String,
    pub output: String,
    pub artifact: ArtifactRef,
    pub exit_code: Option<i32>,
    pub interrupted: bool,
}

pub mod junit;

pub mod playwright;

pub mod vitest;

pub mod jest;
