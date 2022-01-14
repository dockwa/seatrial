use argh::FromArgs;
use rlua::Lua;
use ureq::{Agent, AgentBuilder};
use url::Url;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

mod config_duration;
mod grunt;
mod persona;
mod pipeline;
mod pipeline_action;
mod situation;
mod step_goto;
mod step_http;
mod step_lua;

use crate::grunt::Grunt;
use crate::persona::Persona;
use crate::pipeline::{PipeContents, StepCompletion, StepError};
use crate::pipeline_action::PipelineAction;
use crate::situation::{Situation, SituationSpec};
use crate::step_goto::step as do_step_goto;
use crate::step_http::{
    step_delete as do_step_http_delete, step_get as do_step_http_get,
    step_head as do_step_http_head, step_post as do_step_http_post, step_put as do_step_http_put,
};
use crate::step_lua::step_function as do_step_lua_function;

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
    let barrier = Arc::new(Barrier::new(situations.len()));

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
    let mut lua = Lua::new();

    if let Some(file) = situation.lua_file.as_ref() {
        let fpath = if let Some(parent) = file.parent() {
            let mut ret = parent.to_path_buf();
            ret.push("?.lua");
            ret.to_string_lossy().into_owned()
        } else {
            file.to_string_lossy().into_owned()
        };
        let fname = file
            .file_stem()
            .unwrap_or_else(|| file.as_os_str())
            .to_string_lossy();

        // TODO: something cleaner than unwrap() here
        lua.context(|ctx| {
            ctx.load(&format!(
                "package.path = package.path .. \";{}\"; user_script = require('{}')",
                fpath, fname
            ))
            .set_name(&format!("user_script<{}, {}>", grunt.name, fpath))?
            .exec()
        })
        .unwrap_or_else(|err| {
            eprintln!("[{}] aborting due to lua error", grunt.name);
            eprintln!("[{}] err was: {}", grunt.name, err);
            panic!();
        });
    }

    let persona = &situation.personas[grunt.persona_idx];
    let agent = AgentBuilder::new()
        .timeout((&persona.spec.timeout).into())
        .build();
    let vals = vec![];

    barrier.wait();

    let mut current_pipe_contents: Option<PipeContents> = None;
    let mut current_pipe_idx: usize = 0;
    let mut goto_counters: HashMap<usize, usize> = HashMap::with_capacity(
        persona
            .spec
            .pipeline
            .iter()
            .filter(|step| matches!(step, PipelineAction::GoTo { .. }))
            .count(),
    );

    loop {
        if let Some(step) = &persona.spec.pipeline.get(current_pipe_idx) {
            match do_step(
                step,
                current_pipe_idx,
                &situation.base_url,
                persona,
                &mut lua,
                &agent,
                current_pipe_contents.as_ref(),
                &mut goto_counters,
            ) {
                Ok(StepCompletion::Success {
                    next_index,
                    pipe_data,
                }) => {
                    current_pipe_contents = pipe_data;
                    current_pipe_idx = next_index;
                }
                Ok(StepCompletion::SuccessWithWarnings {
                    next_index,
                    pipe_data,
                }) => {
                    // TODO: log event for warnings
                    current_pipe_contents = pipe_data;
                    current_pipe_idx = next_index;
                }
                Err(StepError::Unclassified) => {
                    eprintln!(
                        "[{}] aborting due to unclassified error in pipeline",
                        grunt.name
                    );
                    eprintln!(
                        "[{}] this is an error in seatrial - TODO fix this",
                        grunt.name
                    );
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
                Err(StepError::InvalidActionInContext) => {
                    eprintln!(
                        "[{}] aborting due to invalid action definition in the given context",
                        grunt.name
                    );
                    eprintln!(
                        "[{}] that this was not caught in a linter run is an error in seatrial - TODO fix this",
                        grunt.name
                    );
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
                Err(StepError::LuaException(err)) => {
                    eprintln!("[{}] aborting due to lua error", grunt.name);
                    eprintln!("[{}] err was: {}", grunt.name, err);
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
                Err(StepError::UrlParsing(err)) => {
                    eprintln!("[{}] aborting due to url parsing error", grunt.name);
                    eprintln!("[{}] err was: {}", grunt.name, err);
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
                Err(StepError::Http(err)) => {
                    eprintln!("[{}] aborting due to http error", grunt.name);
                    eprintln!("[{}] err was: {}", grunt.name, err);
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
            }
        } else {
            eprintln!("[{}] reached end of pipeline, goodbye!", grunt.name);
            break;
        }
    }

    for val in vals {
        tx.send(val).unwrap();
        thread::sleep(Duration::from_secs(1));
    }
}

fn do_step(
    step: &PipelineAction,
    idx: usize,
    base_url: &Url,
    persona: &Persona,
    lua: &mut Lua,
    agent: &Agent,
    _last: Option<&PipeContents>,
    goto_counters: &mut HashMap<usize, usize>,
) -> Result<StepCompletion, StepError> {
    match step {
        PipelineAction::GoTo { index, max_times } => {
            do_step_goto(*index, *max_times, persona, goto_counters)
        }
        PipelineAction::LuaTableIndex(..)
        | PipelineAction::LuaTableValue(..)
        | PipelineAction::LuaValue => Err(StepError::InvalidActionInContext),
        PipelineAction::LuaFunction(fname) => do_step_lua_function(idx, fname, lua),
        PipelineAction::Delete {
            url,
            headers,
            params,
            timeout,
        } => do_step_http_delete(
            idx,
            base_url,
            url,
            headers.as_ref(),
            params.as_ref(),
            timeout.as_ref(),
            agent,
        ),
        PipelineAction::Get {
            url,
            headers,
            params,
            timeout,
        } => do_step_http_get(
            idx,
            base_url,
            url,
            headers.as_ref(),
            params.as_ref(),
            timeout.as_ref(),
            agent,
        ),
        PipelineAction::Head {
            url,
            headers,
            params,
            timeout,
        } => do_step_http_head(
            idx,
            base_url,
            url,
            headers.as_ref(),
            params.as_ref(),
            timeout.as_ref(),
            agent,
        ),
        PipelineAction::Post {
            url,
            headers,
            params,
            timeout,
        } => do_step_http_post(
            idx,
            base_url,
            url,
            headers.as_ref(),
            params.as_ref(),
            timeout.as_ref(),
            agent,
        ),
        PipelineAction::Put {
            url,
            headers,
            params,
            timeout,
        } => do_step_http_put(
            idx,
            base_url,
            url,
            headers.as_ref(),
            params.as_ref(),
            timeout.as_ref(),
            agent,
        ),
        // TODO: remove
        _ => Ok(StepCompletion::Success {
            next_index: idx + 1,
            pipe_data: None,
        }),
    }
}
