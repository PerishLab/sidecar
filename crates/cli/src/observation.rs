use plumb::observation::{Declaration, Run};
use std::path::PathBuf;

pub(crate) fn start(home: PathBuf, command: &str) -> Option<Run> {
    Run::start(
        Declaration {
            producer: "sidecar",
            file: &home.join("config.toml"),
            buffer: home.join("state/locus"),
        },
        command,
    )
}
