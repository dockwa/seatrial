use nanoserde::DeRon;
use ureq::{Agent, AgentBuilder};

use std::collections::HashMap;

use crate::config_duration::ConfigDuration;
use crate::persona::Persona;
use crate::pipeline::action::{ConfigActionMap, PipelineAction};
use crate::pipeline::step_handler::{
    StepCompletion, StepError, StepHandler, StepHandlerInit, StepResult,
};
use crate::pipeline::Pipeline;

#[derive(Clone, Debug, DeRon)]
pub enum Action {
    // http verbs. this section could be fewer LOC with macros eg
    // https://stackoverflow.com/a/37007315/17630058, but (1) this is still manageable (there's
    // only a few HTTP verbs), and (2) rust macros are cryptic enough to a passer-by that if we're
    // going to introduce them and their mental overhead to this codebase (other than depending on
    // a few from crates), we should have a strong reason (and perhaps multiple usecases).

    // TODO: figure out what, if anything, are appropriate guardrails for a PATCH verb
    Delete {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Get {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Head {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Post {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
    Put {
        url: String,
        headers: Option<ConfigActionMap>,
        params: Option<ConfigActionMap>,
        timeout: Option<ConfigDuration>,
    },
}

impl Action {
    pub fn url(&self) -> &String {
        match self {
            Self::Delete { url, .. }
            | Self::Get { url, .. }
            | Self::Head { url, .. }
            | Self::Post { url, .. }
            | Self::Put { url, .. } => url,
        }
    }

    pub fn headers(&self) -> Option<&ConfigActionMap> {
        match self {
            Self::Delete { headers, .. }
            | Self::Get { headers, .. }
            | Self::Head { headers, .. }
            | Self::Post { headers, .. }
            | Self::Put { headers, .. } => headers.as_ref(),
        }
    }

    pub fn params(&self) -> Option<&ConfigActionMap> {
        match self {
            Self::Delete { params, .. }
            | Self::Get { params, .. }
            | Self::Head { params, .. }
            | Self::Post { params, .. }
            | Self::Put { params, .. } => params.as_ref(),
        }
    }

    pub fn timeout(&self) -> Option<&ConfigDuration> {
        match self {
            Self::Delete { timeout, .. }
            | Self::Get { timeout, .. }
            | Self::Head { timeout, .. }
            | Self::Post { timeout, .. }
            | Self::Put { timeout, .. } => timeout.as_ref(),
        }
    }
}
#[derive(Debug)]
pub struct HttpHandler {
    agent: Agent,
}

impl StepHandler for HttpHandler {
    fn new(grunt_name: &str, persona: &Persona) -> StepHandlerInit<Self> {
        Ok(Self {
            agent: AgentBuilder::new()
                .user_agent(&format!(
                    "seatrial/grunt={}/persona={}",
                    grunt_name, persona.name
                ))
                .timeout((&persona.spec.timeout).into())
                .build(),
        })
    }

    fn step(&self, pl: &Pipeline, action: &PipelineAction) -> StepResult {
        match action {
            PipelineAction::Http(verb) => {
                let path = pl
                    .base_url
                    .join(verb.url())
                    .map_err(StepError::UrlParsing)
                    .map(|url| url.to_string())?;

                let mut req = match verb {
                    Action::Delete { .. } => self.agent.delete(&path),
                    Action::Get { .. } => self.agent.get(&path),
                    Action::Head { .. } => self.agent.head(&path),
                    Action::Post { .. } => self.agent.post(&path),
                    Action::Put { .. } => self.agent.put(&path),
                };

                if let Some(timeout) = verb.timeout() {
                    req = req.timeout(timeout.into())
                }

                for (key, val) in self.build_request_hashmap(pl, verb.headers())? {
                    req = req.set(&key, &val);
                }

                for (key, val) in self.build_request_hashmap(pl, verb.params())? {
                    req = req.query(&key, &val);
                }

                req.call()
                    .and_then(|response| Ok(StepCompletion::Normal(Some(response.try_into()?))))
                    .or_else(|err| match err {
                        ureq::Error::Status(_, response) => {
                            Ok(StepCompletion::Normal(Some(response.try_into()?)))
                        }
                        ureq::Error::Transport(_) => Err(StepError::Http(err)),
                    })
            }

            _ => unreachable!("HttpHandler only handles HTTP, Pipeline should have guarded"),
        }
    }
}

impl HttpHandler {
    fn build_request_hashmap(
        &self,
        pl: &Pipeline,
        base_spec: Option<&ConfigActionMap>,
    ) -> Result<HashMap<String, String>, StepError> {
        if let Some(base) = base_spec {
            let mut ret = HashMap::with_capacity(base.len());

            for (key, href) in base {
                ret.insert(
                    key.clone(),
                    // TODO: is there a cleaner way to do this than reaching into LuaForPipeline?
                    href.try_into_string_given_pipe_data(pl.lua, pl.data.as_ref())?,
                );
            }

            Ok(ret)
        } else {
            Ok(HashMap::new())
        }
    }
}
