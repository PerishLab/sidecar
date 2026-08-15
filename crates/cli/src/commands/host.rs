use serde_json::Value;
use std::io::Write;
use std::process::{Child, Command, Stdio};

pub(crate) fn host(command: &[String]) -> Result<(), String> {
    match raise(command) {
        Ok(mut child) => {
            say(&serde_json::json!({ "pid": child.id() }));
            child
                .wait()
                .map_err(|err| format!("failed to wait for the target: {err}"))?;
            Ok(())
        }
        Err(err) => {
            say(&serde_json::json!({ "error": err }));
            Err(err)
        }
    }
}

fn raise(command: &[String]) -> Result<Child, String> {
    let Some((program, args)) = command.split_first() else {
        return Err("runtime host requires `-- <command> [args...]`".to_string());
    };
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(sink()?)
        .stderr(sink()?)
        .spawn()
        .map_err(|err| format!("failed to spawn `{program}`: {err}"))
}

fn sink() -> Result<Stdio, String> {
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        std::io::stderr()
            .as_fd()
            .try_clone_to_owned()
            .map(Stdio::from)
            .map_err(handle)
    }

    #[cfg(windows)]
    {
        use std::os::windows::io::AsHandle;
        std::io::stderr()
            .as_handle()
            .try_clone_to_owned()
            .map(Stdio::from)
            .map_err(handle)
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err("runtime host is not implemented on this platform".to_string())
    }
}

#[cfg(any(unix, windows))]
fn handle(err: std::io::Error) -> String {
    format!("failed to clone the inherited log handle: {err}")
}

fn say(word: &Value) {
    println!("{word}");
    let _ = std::io::stdout().flush();
}
