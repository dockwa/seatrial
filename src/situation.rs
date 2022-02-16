use nanoserde::{DeRon, DeRonErr};
use url::Url;

use std::ffi::OsString;
use std::fs::{canonicalize, read_to_string};
use std::path::PathBuf;
use std::str::FromStr;

use crate::grunt::{Grunt, GruntSpec};

// built out of a SituationSpec after post-parse contextual validations have been run
#[derive(Clone, Debug)]
pub struct Situation {
    pub base_url: Url,
    pub lua_file: Option<PathBuf>,
    pub grunts: Vec<Grunt>,
}

impl Situation {
    pub fn from_spec(
        spec: &SituationSpec,
        base_url: &Url,
        grunt_multiplier: usize,
    ) -> Result<Self, SituationParseErr> {
        let grunts = {
            let mut grunts: Vec<Grunt> = Vec::new();

            for grunt_spec in spec.contents.grunts.iter() {
                grunts.extend(Grunt::from_spec_with_multiplier(
                    grunt_spec,
                    grunt_multiplier,
                )?);
            }

            grunts
        };

        Ok(Self {
            grunts,
            base_url: base_url.clone(),
            lua_file: spec.canonical_lua_file(),
        })
    }
}

#[derive(Clone, Debug, DeRon)]
pub struct SituationSpec {
    source: String,
    contents: SituationSpecContents,
}

impl SituationSpec {
    pub fn canonical_lua_file(&self) -> Option<PathBuf> {
        // this attempts to canonicalize a given string, presuming it's a path to a file.
        // if that fails, it will just pass the given string through to lua unchanged
        // (perhaps we're requiring a lua library from elsewhere in the search path, or a
        // native sofile, or whatever). Nones get passed all the way through, skipping the
        // entire song and dance
        self.contents.lua_file.as_ref().map(|file| {
            canonicalize({
                let mut rel_base = PathBuf::from(&self.source);
                rel_base.pop();
                rel_base.push(file);
                rel_base
            }).map_or_else(|_| {
                eprintln!("[situation parser] error canonicalizing provided lua_file \"{}\" to a path, passing through to lua unmodified", file);

                PathBuf::from(file)
            }, |pb| pb)
        })
    }
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
}

#[derive(Debug)]
pub struct SituationParseErr {
    pub kind: SituationParseErrKind,
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
