use nanoserde::DeRon;

use std::fmt::Display;

#[cfg(test)]
use crate::config_duration::ConfigDuration;
use crate::persona::{Persona, PersonaSpec};
use crate::situation::{SituationParseErr, SituationParseErrKind};

// build out of a GruntSpec during Situation construction
#[derive(Clone, Debug)]
pub struct Grunt {
    pub name: String,
    pub persona: Persona,
}

impl Grunt {
    pub fn from_spec_with_multiplier(
        spec: &GruntSpec,
        multiplier: usize,
    ) -> Result<Vec<Self>, SituationParseErr> {
        let num_grunts = spec.real_count() * multiplier;
        if num_grunts < 1 {
            return Err(SituationParseErr {
                kind: SituationParseErrKind::Semantics {
                    message: "if provided, grunt count must be >=1".into(),
                    location: "unknown".into(), // this gets replaced upstream
                },
            });
        }

        let mut grunts: Vec<Self> = Vec::with_capacity(num_grunts);
        for slot in 0..num_grunts {
            grunts.push(Grunt {
                name: spec.formatted_name(slot),
                persona: (&spec.persona).into(),
            });
        }

        Ok(grunts)
    }
}

#[derive(Clone, Debug, DeRon)]
pub struct GruntSpec {
    pub base_name: Option<String>,
    pub persona: PersonaSpec,
    pub count: Option<usize>,
}

impl GruntSpec {
    pub fn formatted_name(&self, uniqueness: impl Display) -> String {
        format!(
            "{} {}",
            self.base_name.clone().unwrap_or_else(|| format!(
                "Grunt<taking {} actions>",
                self.persona.sequence.len()
            )),
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
        persona: PersonaSpec {
            headers: None,
            sequence: vec![],
            timeout: ConfigDuration::Seconds(30),
        },
        count: None,
    };

    assert_eq!("Jimbo Gruntseph 1", spec.formatted_name(1));
}

#[test]
fn test_formatted_name_no_base() {
    let spec = GruntSpec {
        base_name: None,
        persona: PersonaSpec {
            headers: None,
            sequence: vec![],
            timeout: ConfigDuration::Seconds(30),
        },
        count: None,
    };

    assert_eq!("Grunt<taking 0 actions> 1", spec.formatted_name(1));
}

#[test]
fn test_real_count() {
    let spec = GruntSpec {
        base_name: None,
        persona: PersonaSpec {
            headers: None,
            sequence: vec![],
            timeout: ConfigDuration::Seconds(30),
        },
        count: None,
    };

    assert_eq!(1, spec.real_count());
}
