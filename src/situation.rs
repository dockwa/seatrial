use nanoserde::{DeRon, DeRonErr};
use url::Url;

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{canonicalize, read_to_string};
use std::path::PathBuf;
use std::str::FromStr;

use crate::grunt::{Grunt, GruntSpec};
use crate::persona::{Persona, PersonaSpec};

// built out of a SituationSpec after post-parse contextual validations have been run
#[derive(Clone, Debug)]
pub struct Situation {
    pub base_url: Url,
    pub lua_file: Option<PathBuf>,
    pub grunts: Vec<Grunt>,
    pub personas: Vec<Persona>,
}

impl Situation {
    pub fn from_spec(
        spec: &SituationSpec,
        base_url: &Url,
        grunt_multiplier: usize,
    ) -> Result<Self, SituationParseErr> {
        let mut relocated_personas: HashMap<&str, usize> =
            HashMap::with_capacity(spec.contents.personas.len());
        let personas = spec
            .contents
            .personas
            .iter()
            .enumerate()
            .map(|(idx, (name, spec))| {
                relocated_personas.insert(name, idx);

                Persona {
                    name: name.into(),
                    spec: spec.clone(),

                    // TODO: populate with Value/LuaFunction returns, error on other PipelineAction
                    // variants
                    headers: HashMap::new(),
                }
            })
            .collect();
        let grunts = {
            let mut slot: usize = 0;
            let mut grunts: Vec<Grunt> = Vec::with_capacity(
                spec.contents
                    .grunts
                    .iter()
                    .map(|grunt| grunt.real_count() * grunt_multiplier)
                    .sum(),
            );

            for (idx, grunt_spec) in spec.contents.grunts.iter().enumerate() {
                let num_grunts = grunt_spec.real_count();
                if num_grunts < 1 {
                    return Err(SituationParseErr {
                        kind: SituationParseErrKind::Semantics {
                            message: "if provided, grunt count must be >=1".into(),
                            location: format!("grunts[{}]", idx),
                        },
                    });
                }
                let num_grunts = num_grunts * grunt_multiplier;

                match relocated_personas.get(&*grunt_spec.persona) {
                    Some(persona_idx) => {
                        for _ in 0..num_grunts {
                            grunts.push(Grunt {
                                name: grunt_spec.formatted_name(slot),
                                persona_idx: *persona_idx,
                            });
                            slot += 1;
                        }
                    }
                    None => {
                        return Err(SituationParseErr {
                            kind: SituationParseErrKind::Semantics {
                                message: format!(
                                    "grunt refers to non-existent persona \"{}\"",
                                    grunt_spec.persona
                                ),
                                location: format!("grunts[{}]", idx),
                            },
                        });
                    }
                }
            }

            grunts
        };

        Ok(Self {
            base_url: base_url.clone(),
            lua_file: {
                // this comical chain attempts to canonicalize a given string, presuming it's a
                // path to a file. if that fails, it will just pass the given string through to lua
                // unchanged (perhaps we're requiring a lua library from elsewhere in the search
                // path, or a native sofile, or whatever). Nones get passed all the way through,
                // skipping the entire song and dance
                spec.contents.lua_file.as_ref().map(|file| {
                    let rel_path = {
                        let mut rel_base = PathBuf::from(&spec.source);
                        rel_base.pop();
                        rel_base.push(file);
                        rel_base
                    };

                    eprintln!("rel_path: {:?}", rel_path);

                    let canon = canonicalize(rel_path);

                    if let Ok(path) = canon {
                        path
                    } else {
                        if let Some(provided) = spec.contents.lua_file.as_ref() {
                            eprintln!("[situation parser] error canonicalizing provided lua_file \"{}\" to a path, passing through to lua unmodified", provided);
                        }

                        PathBuf::from(file)
                    }
                })
            },
            grunts,
            personas,
        })
    }
}

#[derive(Clone, Debug, DeRon)]
pub struct SituationSpec {
    source: String,
    contents: SituationSpecContents,
}

impl FromStr for SituationSpec {
    type Err = SituationParseErr;

    fn from_str(it: &str) -> Result<Self, Self::Err> {
        let source = canonicalize(it)?;
        Ok(Self {
            contents: DeRon::deserialize_ron(&read_to_string(&source)?)?,
            source: source.into_os_string().into_string()?,
        })
    }
}

#[derive(Clone, Debug, DeRon)]
pub struct SituationSpecContents {
    lua_file: Option<String>,
    grunts: Vec<GruntSpec>,
    personas: HashMap<String, PersonaSpec>,
}

#[derive(Debug)]
pub struct SituationParseErr {
    kind: SituationParseErrKind,
}

#[derive(Debug)]
pub enum SituationParseErrKind {
    Inspecific(OsString),
    IO(std::io::Error),
    Parsing(DeRonErr),
    Semantics {
        message: String,
        location: String, // should we try to refer back to line numbers in the config somehow?
    },
}

impl SituationParseErr {
    pub fn message(&self) -> String {
        match &self.kind {
            SituationParseErrKind::Inspecific(msg) => msg.to_string_lossy().into(),
            SituationParseErrKind::IO(err) => err.to_string(),
            SituationParseErrKind::Parsing(err) => err.to_string(),
            SituationParseErrKind::Semantics { message, .. } => message.clone(),
        }
    }
}

impl std::fmt::Display for SituationParseErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SituationParseErr: {}", self.message())
    }
}

impl From<std::io::Error> for SituationParseErr {
    fn from(src: std::io::Error) -> Self {
        Self {
            kind: SituationParseErrKind::IO(src),
        }
    }
}

impl From<DeRonErr> for SituationParseErr {
    fn from(src: DeRonErr) -> Self {
        Self {
            kind: SituationParseErrKind::Parsing(src),
        }
    }
}

impl From<OsString> for SituationParseErr {
    fn from(src: OsString) -> Self {
        Self {
            kind: SituationParseErrKind::Inspecific(src),
        }
    }
}
