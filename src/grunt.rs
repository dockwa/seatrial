use nanoserde::DeRon;

use std::fmt::Display;

// build out of a GruntSpec during Situation construction
#[derive(Clone, Debug)]
pub struct Grunt {
    pub name: String,
    pub persona_idx: usize,
}

#[derive(Clone, Debug, DeRon)]
pub struct GruntSpec {
    pub base_name: Option<String>,
    pub persona: String,
    pub count: Option<usize>,
}

impl GruntSpec {
    pub fn formatted_name(&self, uniqueness: impl Display) -> String {
        format!(
            "{} {}",
            self.base_name
                .clone()
                .unwrap_or_else(|| format!("Grunt<{}>", self.persona)),
            uniqueness,
        )
    }

    pub fn real_count(&self) -> usize {
        self.count.unwrap_or(1)
    }
}
