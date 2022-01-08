use nanoserde::DeRon;

use std::time::Duration;

#[derive(Clone, Debug, DeRon)]
pub enum ConfigDuration {
    Milliseconds(u64),
    Seconds(u64),
}

impl From<ConfigDuration> for Duration {
    fn from(src: ConfigDuration) -> Self {
        (&src).into()
    }
}

impl From<&ConfigDuration> for Duration {
    fn from(src: &ConfigDuration) -> Self {
        match src {
            ConfigDuration::Milliseconds(ms) => Duration::from_millis(*ms),
            ConfigDuration::Seconds(ms) => Duration::from_secs(*ms),
        }
    }
}

#[test]
fn test_seconds() {
    assert_eq!(Duration::from_secs(10), ConfigDuration::Seconds(10).into());
}

#[test]
fn test_milliseconds() {
    assert_eq!(
        Duration::from_millis(100),
        ConfigDuration::Milliseconds(100).into()
    );
}
