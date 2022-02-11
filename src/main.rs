use url::Url;

use std::str::FromStr;
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::thread::JoinHandle;

mod cli;
mod combinator;
mod config_duration;
mod grunt;
mod http;
mod http_response_table;
mod lua;
mod persona;
mod pipe_contents;
mod pipeline;
mod situation;
mod validator;

use crate::cli::parse_args;
use crate::grunt::Grunt;
use crate::lua::LuaForPipeline;
use crate::pipeline::step_handler::{StepError, StepHandlerInitError};
use crate::pipeline::{Pipeline, PipelineStepResult};
use crate::situation::Situation;

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

            // does this even do anything to hold the thread open?
            for _ in rx {}
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
    tx: mpsc::Sender<()>,
) -> Result<(), StepHandlerInitError> {
    if situation.lua_file.is_none() {
        unimplemented!("situations without 'lua_file' are not currently supported");
    }

    // TODO: no final unwrap. lua_file.unwrap() is guarded above.
    let lua = LuaForPipeline::new(situation.lua_file.as_ref().unwrap()).unwrap_or_else(
        |err: rlua::Error| {
            eprintln!("[{}] aborting due to lua error", grunt.name);
            eprintln!("[{}] err was: {}", grunt.name, err);
            panic!();
        },
    );

    barrier.wait();

    for step_result in Pipeline::new(
        &grunt.name,
        &situation.base_url,
        &situation.personas[grunt.persona_idx],
        Some(&lua),
    )? {
        match step_result {
            Ok(PipelineStepResult::Ok) => {}

            Ok(PipelineStepResult::OkWithWarnings(warnings)) => {
                // TODO: in addition to printing, we need to track structured events (not just
                // for these warnings, but for all sorts of pipeline actions)

                for warning in warnings {
                    eprintln!(
                        "[{}] warning issued during pipeline step completion: {}",
                        grunt.name, warning
                    );
                }
            }

            Ok(PipelineStepResult::OkWithExit) => {
                break;
            }

            Err(err) => {
                process_step_error(grunt, err);
                break;
            }
        }
    }

    grunt_exit(grunt);

    // TODO should probably handle this more UX-sanely
    tx.send(()).unwrap();

    Ok(())
}

fn grunt_exit(grunt: &Grunt) {
    eprintln!("[{}] reached end of pipeline, goodbye!", grunt.name);
}

fn process_step_error(grunt: &Grunt, err: StepError) {
    match err {
        StepError::Unclassified => {
            eprintln!(
                "[{}] aborting due to unclassified error in pipeline",
                grunt.name
            );
            eprintln!(
                "[{}] this is an error in seatrial - TODO fix this",
                grunt.name
            );
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::Validation(err) => {
            eprintln!(
                "[{}] aborting due to validation error in pipeline",
                grunt.name
            );
            eprintln!("[{}] err was: {}", grunt.name, err);
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        // TODO: more details - we're just not plumbing the details around
        StepError::ValidationSucceededUnexpectedly => {
            eprintln!(
                "[{}] aborting because a validation succeeded where we expected a failure",
                grunt.name
            );
            eprintln!(
                "[{}] this is an error in seatrial - TODO fix this",
                grunt.name
            );
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::InvalidActionInContext => {
            eprintln!(
                "[{}] aborting due to invalid action definition in the given context",
                grunt.name
            );
            eprintln!(
                "[{}] that this was not caught in a linter run is an error in seatrial - TODO fix this",
                grunt.name
            );
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::IO(err) => {
            eprintln!("[{}] aborting due to internal IO error", grunt.name);
            eprintln!("[{}] err was: {}", grunt.name, err);
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::LuaException(err) => {
            eprintln!("[{}] aborting due to lua error", grunt.name);
            eprintln!("[{}] err was: {}", grunt.name, err);
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::UrlParsing(err) => {
            eprintln!("[{}] aborting due to url parsing error", grunt.name);
            eprintln!("[{}] err was: {}", grunt.name, err);
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::Http(err) => {
            eprintln!("[{}] aborting due to http error", grunt.name);
            eprintln!("[{}] err was: {}", grunt.name, err);
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::RefuseToStringifyComplexLuaValue => {
            eprintln!(
                "[{}] aborting attempt to stringify complex lua value",
                grunt.name
            );
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::RefuseToStringifyNonExistantValue => {
            eprintln!(
                "[{}] aborting attempt to stringify non-existent (probably nil) lua value",
                grunt.name
            );
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        // TODO: FIXME this messaging is extremely hard to grok, I'd be pounding my head
        // into the keyboard screaming obscenities if a tool offered me this as the sole
        // debug output
        StepError::RequestedLuaValueWhereNoneExists => {
            eprintln!(
                "[{}] aborting attempt to pass non-existent value to lua context",
                grunt.name
            );
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }

        StepError::LuaNotInstantiated => {
            eprintln!(
                "[{}] aborting attempt to use lua when it is not instantiated",
                grunt.name
            );
            // TODO: restore
            //eprintln!("[{}] step was: {:?}", grunt.name, step);
        }
    }
}
