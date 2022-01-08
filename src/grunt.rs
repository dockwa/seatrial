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

#[test]
fn test_formatted_name() {
    let spec = GruntSpec {
        base_name: Some("Jimbo Gruntseph".into()),
        persona: "blahblah".into(),
        count: None,
    };

    assert_eq!("Jimbo Gruntseph 1", spec.formatted_name(1));
}

#[test]
fn test_formatted_name_no_base() {
    let spec = GruntSpec {
        base_name: None,
        persona: "blahblah".into(),
        count: None,
    };

    assert_eq!("Grunt<blahblah> 1", spec.formatted_name(1));
}

#[test]
fn test_real_count() {
    let spec = GruntSpec {
        base_name: None,
        persona: "blahblah".into(),
        count: None,
    };

    assert_eq!(1, spec.real_count());
}
