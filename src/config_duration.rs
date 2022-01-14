use nanoserde::DeRon;

use std::time::Duration;

#[derive(Clone, Debug, DeRon)]
pub enum ConfigDuration {
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
