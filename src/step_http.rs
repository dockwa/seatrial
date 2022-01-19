use ureq::Agent;
use url::Url;

use crate::config_duration::ConfigDuration;
use crate::pipeline::{PipeContents, StepCompletion, StepError};
use crate::pipeline_action::ConfigActionMap;

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
    last: Option<&PipeContents>,
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
    last: Option<&PipeContents>,
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
    last: Option<&PipeContents>,
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
    last: Option<&PipeContents>,
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
    last: Option<&PipeContents>,
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
    last: Option<&PipeContents>,
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
            )
        })
}

fn request_common(
    mut req: ureq::Request,
    timeout: Option<&ConfigDuration>,
    idx: usize,
    headers: Option<&ConfigActionMap>,
    params: Option<&ConfigActionMap>,
    last: Option<&PipeContents>,
) -> Result<StepCompletion, StepError> {
    if let Some(timeout) = timeout {
        req = req.timeout(timeout.into())
    }

    req.call()
        .map(|response| StepCompletion::Normal {
            next_index: idx + 1,
            pipe_data: Some(PipeContents::HttpResponse(response)),
        })
        .or_else(|err| {
            match err {
                ureq::Error::Status(_, response) => Ok(StepCompletion::Normal {
                    next_index: idx + 1,
                    pipe_data: Some(PipeContents::HttpResponse(response)),
                }),
                ureq::Error::Transport(_) => Err(StepError::Http(err)),
        }})
}
