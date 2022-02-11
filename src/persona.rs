use nanoserde::DeRon;

use std::collections::HashMap;

use crate::config_duration::ConfigDuration;
use crate::pipeline::action::{ConfigActionMap, PipelineAction};

// built out of a PersonaSpec during Situation construction
#[derive(Clone, Debug)]
pub struct Persona {
    pub name: String,
    pub spec: PersonaSpec,
    pub headers: HashMap<String, String>,
}

#[derive(Clone, Debug, DeRon)]
pub struct PersonaSpec {
    pub timeout: ConfigDuration,
    pub headers: Option<ConfigActionMap>,
    pub sequence: Vec<PipelineAction>,
}
