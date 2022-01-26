use rlua::Lua;
use ureq::{Agent, Request};
use url::Url;

use std::collections::HashMap;

use crate::config_duration::ConfigDuration;
use crate::pipe_contents::PipeContents as PC;
use crate::pipeline::StepCompletion;
use crate::pipeline_action::ConfigActionMap;
use crate::step_error::StepError;

#[derive(Debug)]
enum Verb {
    Delete,
    Get,
    Head,
    Post,
    Put,
}

pub fn step_delete(
    idx: usize,
    base_url: &Url,
    path: &str,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    timeout: Option<&ConfigDuration>,
    agent: &Agent,
    last: Option<&PC>,
    lua: &Lua,
) -> Result<StepCompletion, StepError> {
    step(
        Verb::Delete,
        idx,
        base_url,
        path,
        headers,
        params,
        timeout,
        agent,
        last,
        lua,
    )
}

pub fn step_get(
    idx: usize,
    base_url: &Url,
    path: &str,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    timeout: Option<&ConfigDuration>,
    agent: &Agent,
    last: Option<&PC>,
    lua: &Lua,
) -> Result<StepCompletion, StepError> {
    step(
        Verb::Get,
        idx,
        base_url,
        path,
        headers,
        params,
        timeout,
        agent,
        last,
        lua,
    )
}

pub fn step_head(
    idx: usize,
    base_url: &Url,
    path: &str,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    timeout: Option<&ConfigDuration>,
    agent: &Agent,
    last: Option<&PC>,
    lua: &Lua,
) -> Result<StepCompletion, StepError> {
    step(
        Verb::Head,
        idx,
        base_url,
        path,
        headers,
        params,
        timeout,
        agent,
        last,
        lua,
    )
}

pub fn step_post(
    idx: usize,
    base_url: &Url,
    path: &str,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    timeout: Option<&ConfigDuration>,
    agent: &Agent,
    last: Option<&PC>,
    lua: &Lua,
) -> Result<StepCompletion, StepError> {
    step(
        Verb::Post,
        idx,
        base_url,
        path,
        headers,
        params,
        timeout,
        agent,
        last,
        lua,
    )
}

pub fn step_put(
    idx: usize,
    base_url: &Url,
    path: &str,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    timeout: Option<&ConfigDuration>,
    agent: &Agent,
    last: Option<&PC>,
    lua: &Lua,
) -> Result<StepCompletion, StepError> {
    step(
        Verb::Put,
        idx,
        base_url,
        path,
        headers,
        params,
        timeout,
        agent,
        last,
        lua,
    )
}

fn step(
    verb: Verb,
    idx: usize,
    base_url: &Url,
    path: &str,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    timeout: Option<&ConfigDuration>,
    agent: &Agent,
    last: Option<&PC>,
    lua: &Lua,
) -> Result<StepCompletion, StepError> {
    base_url
        .join(path)
        .map_err(StepError::UrlParsing)
        .and_then(|url| {
            let stringified_path = &url.to_string();

            request_common(
                match verb {
                    Verb::Delete => agent.delete(stringified_path),
                    Verb::Get => agent.get(stringified_path),
                    Verb::Head => agent.head(stringified_path),
                    Verb::Post => agent.post(stringified_path),
                    Verb::Put => agent.put(stringified_path),
                },
                timeout,
                idx,
                headers,
                params,
                last,
                lua,
            )
        })
}

fn request_common(
    mut req: Request,
    timeout: Option<&ConfigDuration>,
    idx: usize,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    last: Option<&PC>,
    lua: &Lua,
) -> Result<StepCompletion, StepError> {
    if let Some(timeout) = timeout {
        req = req.timeout(timeout.into())
    }

    for (key, val) in build_request_hashmap(headers, lua, last)? {
        req = req.set(&key, &val);
    }

    for (key, val) in build_request_hashmap(params, lua, last)? {
        req = req.query(&key, &val);
    }

    req.call()
        .and_then(|response| {
            Ok(StepCompletion::Normal {
                next_index: idx + 1,
                pipe_data: Some(response.try_into()?),
            })
        })
        .or_else(|err| match err {
            ureq::Error::Status(_, response) => Ok(StepCompletion::Normal {
                next_index: idx + 1,
                pipe_data: Some(response.try_into()?),
            }),
            ureq::Error::Transport(_) => Err(StepError::Http(err)),
        })
}

fn build_request_hashmap(
    base_spec: Option<&ConfigActionMap>,
    lua: &Lua,
    pipe_data: Option<&PC>,
) -> Result<HashMap<String, String>, StepError> {
    if let Some(base) = base_spec {
        let mut ret = HashMap::with_capacity(base.len());

        for (key, href) in base {
            ret.insert(key.clone(), href.try_into_string_given_pipe_data(lua, pipe_data)?);
        }

        Ok(ret)
    } else {
        Ok(HashMap::new())
    }
}
