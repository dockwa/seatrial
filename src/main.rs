use rlua::{Lua, RegistryKey};
use ureq::{Agent, AgentBuilder};
use url::Url;

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

mod cli;
mod config_duration;
mod grunt;
mod http_response_table;
mod persona;
mod pipe_contents;
mod pipeline;
mod pipeline_action;
mod shared_lua;
mod situation;
mod step_combinator;
mod step_error;
mod step_goto;
mod step_http;
mod step_lua;
mod step_validator;

use crate::cli::parse_args;
use crate::grunt::Grunt;
use crate::persona::Persona;
use crate::pipe_contents::PipeContents;
use crate::pipeline::StepCompletion;
use crate::pipeline_action::{ControlFlow, Http, PipelineAction as PA, Reference};
use crate::shared_lua::attach_seatrial_stdlib;
use crate::situation::Situation;
use crate::step_combinator::step as do_step_combinator;
use crate::step_error::{StepError, StepResult};
use crate::step_goto::step as do_step_goto;
use crate::step_http::{
    step_delete as do_step_http_delete, step_get as do_step_http_get,
    step_head as do_step_http_head, step_post as do_step_http_post, step_put as do_step_http_put,
};
use crate::step_lua::step_function as do_step_lua_function;
use crate::step_validator::step as do_step_validator;

fn main() -> std::io::Result<()> {
    let args = parse_args();

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
    let lua = Lua::default();
    // TODO: no unwrap
    attach_seatrial_stdlib(&lua).unwrap();

    let user_script_registry_key = situation
        .lua_file
        .as_ref()
        .map(|file| {
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
                    "package.path = package.path .. \";{}\"; _user_script = require('{}')",
                    fpath, fname
                ))
                .set_name(&format!("user_script<{}, {}>", grunt.name, fpath))?
                .exec()?;

                let user_script = ctx.globals().get::<_, rlua::Table>("_user_script")?;

                Ok(ctx
                    .create_registry_value(user_script)
                    .expect("should have stored user script in registry"))
            })
            .unwrap_or_else(|err: rlua::Error| {
                eprintln!("[{}] aborting due to lua error", grunt.name);
                eprintln!("[{}] err was: {}", grunt.name, err);
                panic!();
            })
        })
        .unwrap(); // TODO: remove and handle non-extant case

    let persona = &situation.personas[grunt.persona_idx];
    let agent = AgentBuilder::new()
        .user_agent(&format!(
            "seatrial/grunt={}/persona={}",
            grunt.name, persona.name
        ))
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
            .filter(|step| matches!(step, PA::ControlFlow(ControlFlow::GoTo { .. })))
            .count(),
    );

    loop {
        if let Some(step) = &persona.spec.pipeline.get(current_pipe_idx) {
            match do_step(
                step,
                current_pipe_idx,
                &situation.base_url,
                persona,
                &lua,
                &user_script_registry_key,
                &agent,
                current_pipe_contents.as_ref(),
                &mut goto_counters,
            ) {
                Ok(StepCompletion::WithExit) => {
                    grunt_exit(grunt);
                    break;
                }
                Ok(StepCompletion::Normal {
                    next_index,
                    pipe_data,
                }) => {
                    current_pipe_contents = pipe_data;
                    current_pipe_idx = next_index;
                }
                Ok(StepCompletion::WithWarnings {
                    next_index,
                    pipe_data,
                    warnings,
                }) => {
                    // TODO: in addition to printing, we need to track structured events (not just
                    // for these warnings, but for all sorts of pipeline actions)

                    for warning in warnings {
                        eprintln!(
                            "[{}] warning issued during pipeline step completion: {}",
                            grunt.name, warning
                        );
                    }

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
                Err(StepError::Validation(err)) => {
                    eprintln!(
                        "[{}] aborting due to validation error in pipeline",
                        grunt.name
                    );
                    eprintln!("[{}] err was: {}", grunt.name, err);
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
                // TODO: more details - we're just not plumbing the details around
                Err(StepError::ValidationSucceededUnexpectedly) => {
                    eprintln!(
                        "[{}] aborting because a validation succeeded where we expected a failure",
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
                Err(StepError::IO(err)) => {
                    eprintln!("[{}] aborting due to internal IO error", grunt.name);
                    eprintln!("[{}] err was: {}", grunt.name, err);
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
                Err(StepError::RefuseToStringifyComplexLuaValue) => {
                    eprintln!(
                        "[{}] aborting attempt to stringify complex lua value",
                        grunt.name
                    );
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
                // TODO: FIXME this messaging is extremely hard to grok, I'd be pounding my head
                // into the keyboard screaming obscenities if a tool offered me this as the sole
                // debug output
                Err(StepError::RequestedLuaValueWhereNoneExists) => {
                    eprintln!(
                        "[{}] aborting attempt to pass non-existent value to lua context",
                        grunt.name
                    );
                    eprintln!("[{}] step was: {:?}", grunt.name, step);
                    break;
                }
            }
        } else {
            grunt_exit(grunt);
            break;
        }
    }

    for val in vals {
        tx.send(val).unwrap();
        thread::sleep(Duration::from_secs(1));
    }
}

fn grunt_exit(grunt: &Grunt) {
    eprintln!("[{}] reached end of pipeline, goodbye!", grunt.name);
}

fn do_step<'a>(
    step: &PA,
    idx: usize,
    base_url: &Url,
    persona: &Persona,

    // TODO: merge into a combo struct
    lua: &'a Lua,
    user_script_registry_key: &'a RegistryKey,

    agent: &Agent,
    last: Option<&PipeContents>,
    goto_counters: &mut HashMap<usize, usize>,
) -> StepResult {
    match step {
        PA::ControlFlow(ControlFlow::GoTo { index, max_times }) => {
            if let Some(times) = max_times {
                if *times == 0 {
                    // TODO: should probably warn here, or just outright disallow this (either by a
                    // bounded integral type rather than usize, or by failing at lint time)
                    return Ok(StepCompletion::WithExit);
                }

                match goto_counters.get(index) {
                    Some(rem) => {
                        if *rem == 0 {
                            return Ok(StepCompletion::WithExit);
                        }
                    }
                    None => {
                        goto_counters.insert(*index, *times);
                    }
                };

                goto_counters.insert(*index, goto_counters.get(index).unwrap() - 1);
            }

            do_step_goto(*index, persona)
        }

        PA::Reference(Reference::Value(..))
        | PA::Reference(Reference::LuaTableIndex(..))
        | PA::Reference(Reference::LuaTableValue(..))
        | PA::Reference(Reference::LuaValue) => Err(StepError::InvalidActionInContext),

        PA::LuaFunction(fname) => {
            do_step_lua_function(idx, fname, lua, user_script_registry_key, last)
        }

        act @ (PA::Http(Http::Delete {
            url,
            headers,
            params,
            timeout,
        })
        | PA::Http(Http::Get {
            url,
            headers,
            params,
            timeout,
        })
        | PA::Http(Http::Head {
            url,
            headers,
            params,
            timeout,
        })
        | PA::Http(Http::Post {
            url,
            headers,
            params,
            timeout,
        })
        | PA::Http(Http::Put {
            url,
            headers,
            params,
            timeout,
        })) => {
            let method = match act {
                PA::Http(Http::Delete { .. }) => do_step_http_delete,
                PA::Http(Http::Get { .. }) => do_step_http_get,
                PA::Http(Http::Head { .. }) => do_step_http_head,
                PA::Http(Http::Post { .. }) => do_step_http_post,
                PA::Http(Http::Put { .. }) => do_step_http_put,
                _ => unreachable!(),
            };

            method(
                idx,
                base_url,
                url,
                headers.as_ref(),
                params.as_ref(),
                timeout.as_ref(),
                agent,
                last,
                lua,
            )
        }

        PA::Combinator(combo) => {
            do_step_combinator(idx, combo, lua, user_script_registry_key, last)
        }

        PA::Validator(validator) => {
            do_step_validator(idx, validator, lua, user_script_registry_key, last)
        }
    }
}
