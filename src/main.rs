use argh::FromArgs;
use nanoserde::{DeRon, DeRonErr};
use rlua::Lua;
use ureq::AgentBuilder;
use url::Url;

use std::collections::HashMap;
use std::fs::{canonicalize, read_to_string};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

/// situational-mock-based load testing
#[derive(FromArgs)]
struct CmdArgs {
    /// integral multiplier for grunt counts (minimum 1)
    #[argh(option, short = 'm', default = "1")]
    multiplier: usize,

    /// base URL for all situations in this run
    #[argh(positional)]
    base_url: String,

    // work around https://github.com/google/argh/issues/13 wherein repeatable positional arguments
    // (situations, in this struct) allow any vec length 0+, where we require a vec length 1+. this
    // could be hacked around with some From magic and a custom Vec, but this is more
    // straightforward
    /// path to a RON file in seatrial(5) situation config format
    #[argh(positional)]
    req_situation: SituationSpec,

    /// optional paths to additional RON files in seatrial(5) situation config format
    #[argh(positional)]
    situations: Vec<SituationSpec>,
}

// built out of a SituationSpec after post-parse contextual validations have been run
#[derive(Clone, Debug)]
struct Situation {
    base_url: Url,
    lua_file: Option<PathBuf>,
    grunts: Vec<Grunt>,
    personas: Vec<Persona>,
}

impl Situation {
    fn from_spec(
        spec: &SituationSpec,
        base_url: &Url,
        grunt_multiplier: usize,
    ) -> Result<Self, SituationParseErr> {
        let mut relocated_personas: HashMap<&str, usize> =
            HashMap::with_capacity(spec.personas.len());
        let personas = spec
            .personas
            .iter()
            .enumerate()
            .map(|(idx, (name, spec))| {
                relocated_personas.insert(name, idx);

                Persona {
                    name: name.clone(),
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
                spec.grunts
                    .iter()
                    .map(|grunt| grunt.real_count() * grunt_multiplier)
                    .sum(),
            );

            for (idx, grunt_spec) in spec.grunts.iter().enumerate() {
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
            lua_file: spec
                .lua_file
                .as_deref()
                .and_then(|file| canonicalize(file).map(Some).unwrap_or(None)),
            grunts,
            personas,
        })
    }
}

#[derive(Clone, Debug, DeRon)]
struct SituationSpec {
    lua_file: Option<String>,
    grunts: Vec<GruntSpec>,
    personas: HashMap<String, PersonaSpec>,
}

// build out of a GruntSpec during Situation construction
#[derive(Clone, Debug)]
struct Grunt {
    name: String,
    persona_idx: usize,
}

#[derive(Clone, Debug, DeRon)]
struct GruntSpec {
    base_name: Option<String>,
    persona: String,
    count: Option<usize>,
}

impl GruntSpec {
    pub fn formatted_name(&self, uniqueness: impl std::fmt::Display) -> String {
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

// built out of a PersonaSpec during Situation construction
#[derive(Clone, Debug)]
struct Persona {
    name: String,
    spec: PersonaSpec,
    headers: HashMap<String, String>,
}

#[derive(Clone, Debug, DeRon)]
struct PersonaSpec {
    timeout: ConfigDuration,
    headers: Option<HashMap<String, PipelineAction>>,
    pipeline: Vec<PipelineAction>,
}

#[derive(Clone, Debug, DeRon)]
enum ConfigDuration {
    Milliseconds(u64),
    Seconds(u64),
}

impl From<&ConfigDuration> for Duration {
    fn from(src: &ConfigDuration) -> Self {
        match src {
            ConfigDuration::Milliseconds(ms) => Duration::from_millis(*ms),
            ConfigDuration::Seconds(ms) => Duration::from_secs(*ms),
        }
    }
}

#[derive(Clone, Debug, DeRon)]
enum PipelineAction {
    GoTo {
        index: usize,
        max_times: Option<usize>,
    },
    // this is mostly used for URL params, since those _can_ come from Lua, and thus have to be a
    // PipelineAction member
    Value(String),

    // http verbs. this section could be fewer LOC with macros eg
    // https://stackoverflow.com/a/37007315/17630058, but (1) this is still manageable (there's
    // only a few HTTP verbs), and (2) rust macros are cryptic enough to a passer-by that if we're
    // going to introduce them and their mental overhead to this codebase (other than depending on
    // a few from crates), we should have a strong reason (and perhaps multiple usecases).

    // TODO: figure out what, if anything, are appropriate guardrails for a PATCH verb
    Delete {
        url: String,
        headers: Option<HashMap<String, PipelineAction>>,
        params: Option<HashMap<String, PipelineAction>>,
        timeout_ms: Option<u64>,
    },
    Get {
        url: String,
        headers: Option<HashMap<String, PipelineAction>>,
        params: Option<HashMap<String, PipelineAction>>,
        timeout_ms: Option<u64>,
    },
    Head {
        url: String,
        headers: Option<HashMap<String, PipelineAction>>,
        params: Option<HashMap<String, PipelineAction>>,
        timeout_ms: Option<u64>,
    },
    Post {
        url: String,
        headers: Option<HashMap<String, PipelineAction>>,
        params: Option<HashMap<String, PipelineAction>>,
        timeout_ms: Option<u64>,
    },
    Put {
        url: String,
        headers: Option<HashMap<String, PipelineAction>>,
        params: Option<HashMap<String, PipelineAction>>,
        timeout_ms: Option<u64>,
    },

    // validations of whatever the current thing in the pipe is. Asserts are generally fatal when
    // falsey, except in the context of an AnyOf or NoneOf combinator, which can "catch" the errors
    // as appropriate. WarnUnless validations are never fatal and likewise can never fail a
    // combinator
    AssertHeaderExists(String),
    AssertStatusCode(u16),
    AssertStatusCodeInRange(u16, u16),
    WarnUnlessHeaderExists(String),
    WarnUnlessStatusCode(u16),
    WarnUnlessStatusCodeInRange(u16, u16),

    // basic logic. rust doesn't allow something like
    // All(AssertStatusCode|AssertStatusCodeInRange), so instead, **any** PipelineAction is a valid
    // member of a combinator for now, which is less than ideal ergonomically to say the least
    AllOf(Vec<PipelineAction>),
    AnyOf(Vec<PipelineAction>),
    NoneOf(Vec<PipelineAction>),

    // the "Here Be Dragons" section, for when dynamism is absolutely needed: an escape hatch to
    // Lua. TODO: document the Lua APIs and semantics...
    LuaFunction(String),
    LuaValue,
    LuaTableIndex(usize),
    LuaTableValue(String),
}

#[derive(Debug)]
struct SituationParseErr {
    kind: SituationParseErrKind,
}

#[derive(Debug)]
enum SituationParseErrKind {
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

impl FromStr for SituationSpec {
    type Err = SituationParseErr;

    fn from_str(it: &str) -> Result<Self, Self::Err> {
        Ok(DeRon::deserialize_ron(&read_to_string(canonicalize(it)?)?)?)
    }
}

fn main() -> std::io::Result<()> {
    let args = {
        let mut args: CmdArgs = argh::from_env();
        args.situations.insert(0, args.req_situation.clone());
        args
    };

    // TODO: no unwrap, which will also kill the nasty parens
    let base_url = (if args.base_url.ends_with('/') {
        Url::from_str(&args.base_url)
    } else {
        Url::from_str(&format!("{}/", args.base_url))
    })
    .unwrap();

    // TODO: get rid of unwrap!
    let situations: Vec<Arc<Situation>> = args
        .situations
        .iter()
        .map(|situation| {
            Arc::new(Situation::from_spec(situation, &base_url, args.multiplier).unwrap())
        })
        .collect();

    // TODO: find a less hacky way of dealing with situation lifecycles. this is a brute-force
    // "just throw it on the heap until the kernel kills the process when we exit" hackaround
    // to the borrow checker complaining about needing 'static lifespans down in thread-spawn
    // land, which _works_, but feels messy
    let situations = Box::new(situations).leak();

    // no need for any of the ephemeral *Spec objects at this point
    drop(args);

    let mut situation_threads: Vec<JoinHandle<()>> = Vec::with_capacity(situations.len());
    let barrier = Arc::new(Barrier::new(situation_threads.len()));

    for situation in situations {
        let barrier = barrier.clone();

        situation_threads.push(thread::spawn(move || {
            let (tx, rx) = mpsc::channel();

            for grunt in &situation.grunts {
                let barrier = barrier.clone();
                let situation = situation.clone();
                let tx = tx.clone();

                thread::spawn(move || grunt_worker(barrier, situation, grunt, tx));
            }

            // have to drop the original tx to get refcounts correct, else controller thread will
            // hang indefinitely while rx thinks it has potential inbound data
            drop(tx);

            for received in rx {
                println!("Got: {}", received);
            }
        }));
    }

    for thread in situation_threads {
        thread.join().unwrap();
    }

    Ok(())
}

fn grunt_worker(
    barrier: Arc<Barrier>,
    situation: Arc<Situation>,
    grunt: &Grunt,
    tx: mpsc::Sender<String>,
) {
    let lua = Lua::new();
    let persona = &situation.personas[grunt.persona_idx];
    let agent = AgentBuilder::new()
        .timeout((&persona.spec.timeout).into())
        .build();
    let vals = vec![
        String::from("hi"),
        String::from("from"),
        String::from("the"),
        String::from("thread"),
    ];

    barrier.wait();

    for val in vals {
        tx.send(val).unwrap();
        thread::sleep(Duration::from_secs(1));
    }
}
