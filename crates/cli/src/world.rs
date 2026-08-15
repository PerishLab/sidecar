use serde_json::Value;
use std::collections::BTreeMap;
use std::io::IsTerminal;

fn main() {
    let world = serde_json::json!({
        "argv": argv(),
        "env": env(),
        "cwd": cwd(),
        "pid": std::process::id(),
        "ppid": parent(),
        "pgid": group(),
        "tty": tty(),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&world).unwrap_or_default()
    );
}

fn argv() -> Vec<String> {
    std::env::args_os()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn env() -> BTreeMap<String, String> {
    std::env::vars_os()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.to_string_lossy().into_owned(),
            )
        })
        .collect()
}

fn cwd() -> Option<String> {
    std::env::current_dir()
        .ok()
        .map(|path| path.display().to_string())
}

fn parent() -> Option<u32> {
    #[cfg(unix)]
    {
        Some(std::os::unix::process::parent_id())
    }

    #[cfg(not(unix))]
    {
        None
    }
}

fn group() -> Option<i32> {
    #[cfg(unix)]
    {
        Some(unsafe { libc::getpgrp() })
    }

    #[cfg(not(unix))]
    {
        None
    }
}

fn tty() -> Value {
    serde_json::json!({
        "stdin": std::io::stdin().is_terminal(),
        "stdout": std::io::stdout().is_terminal(),
        "stderr": std::io::stderr().is_terminal(),
    })
}
