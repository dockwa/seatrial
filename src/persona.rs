use nanoserde::DeRon;

use std::collections::HashMap;

use crate::config_duration::ConfigDuration;
use crate::pipeline::action::{ConfigActionMap, PipelineAction};

#[derive(Clone, Debug)]
pub struct Persona {
    pub timeout: ConfigDuration,
    pub headers: HashMap<String, String>,
    pub sequence: Vec<PipelineAction>,
}

impl From<&PersonaSpec> for Persona {
    fn from(spec: &PersonaSpec) -> Self {
        Self {
            timeout: spec.timeout.clone(),
            sequence: spec.sequence.clone(),

            // TODO: populate with Value/LuaFunction returns, error on other PipelineAction
            // variants
            headers: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, DeRon)]
pub struct PersonaSpec {
    pub timeout: ConfigDuration,
    pub headers: Option<ConfigActionMap>,
    pub sequence: Vec<PipelineAction>,
}
