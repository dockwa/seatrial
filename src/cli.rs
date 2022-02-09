use argh::FromArgs;

use crate::situation::SituationSpec;

/// situational-mock-based load testing
#[derive(FromArgs)]
struct CmdArgsBase {
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

#[derive(Clone, Debug)]
pub struct CmdArgs {
    /// integral multiplier for grunt counts (minimum 1)
    pub multiplier: usize,

    /// base URL for all situations in this run
    pub base_url: String,

    /// paths to RON files in seatrial(5) situation config format
    pub situations: Vec<SituationSpec>,
}

/// flatten situations into a single vec (see docs about CmdArgsBase::req_situation)
impl From<CmdArgsBase> for CmdArgs {
    fn from(mut it: CmdArgsBase) -> Self {
        it.situations.insert(0, it.req_situation.clone());

        Self {
            multiplier: it.multiplier,
            base_url: it.base_url,
            situations: it.situations,
        }
    }
}

pub fn parse_args() -> CmdArgs {
    argh::from_env::<CmdArgsBase>().into()
}
