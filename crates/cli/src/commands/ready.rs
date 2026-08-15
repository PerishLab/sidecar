use super::runtime::{running, state};
use serde_json::Value;
use sidecar_core::plan::{Plan, Target};
use sidecar_core::{Paths, process};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
#[cfg(windows)]
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(crate) struct Ready {
    pub(crate) role: String,
    pub(crate) endpoint: Option<String>,
    pub(crate) runtime: Option<String>,
    pub(crate) instance: Option<String>,
}

#[derive(Default)]
pub(crate) struct Chain {
    ready: BTreeMap<String, Ready>,
}

impl Chain {
    pub(crate) fn load(paths: &Paths, plan: &Plan) -> Result<Self, String> {
        let saved = state::load(paths)?;
        let mut chain = Self::default();
        for target in &plan.targets {
            if running(paths, target)?.is_empty() {
                continue;
            }
            let Some(entry) = saved.get(&target.name) else {
                continue;
            };
            let Some(ready) = ready(entry) else {
                continue;
            };
            chain.ready.insert(target.name.clone(), ready);
        }
        Ok(chain)
    }

    pub(crate) fn record(&mut self, name: &str, ready: &Ready) {
        self.ready.insert(name.to_string(), ready.clone());
    }

    pub(crate) fn inherits(&self, target: &Target) -> Result<Vec<(String, String)>, String> {
        let mut env = Vec::new();
        for binding in &target.inherits {
            let Some((source, field)) = binding.from.split_once('.') else {
                return Err(format!(
                    "invalid inherits_env source {:?}; expected '<target>.<field>'",
                    binding.from
                ));
            };
            let value = inherited(self.ready.get(source), field)?;
            if let Some(value) = value {
                env.push((binding.name.clone(), value));
            }
        }
        Ok(env)
    }
}

pub(super) fn inherited(ready: Option<&Ready>, field: &str) -> Result<Option<String>, String> {
    let Some(ready) = ready else {
        return Ok(None);
    };
    match field {
        "endpoint" => Ok(ready.endpoint.clone()),
        "runtime_endpoint" => Ok(ready.runtime.clone()),
        "instance_id" => Ok(ready.instance.clone()),
        other => Err(format!(
            "invalid inherits_env field {other:?}; expected endpoint|runtime_endpoint|instance_id"
        )),
    }
}

pub(super) fn ready(entry: &Value) -> Option<Ready> {
    let ready = entry.get("ready")?;
    Some(Ready {
        role: ready
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        endpoint: ready
            .get("endpoint")
            .and_then(Value::as_str)
            .map(str::to_string),
        runtime: ready
            .get("runtimeEndpoint")
            .and_then(Value::as_str)
            .map(str::to_string),
        instance: ready
            .get("instanceId")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

pub(super) fn scan(log: &Path, expected: &str) -> Result<Option<Ready>, String> {
    let Ok(content) = fs::read_to_string(log) else {
        return Ok(None);
    };
    for line in content.lines().rev() {
        let Ok(value) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let role = value
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if role != expected {
            continue;
        }
        return Ok(Some(Ready {
            role: role.to_string(),
            endpoint: value
                .get("endpoint")
                .and_then(Value::as_str)
                .map(str::to_string),
            runtime: value
                .get("runtime_endpoint")
                .and_then(Value::as_str)
                .map(str::to_string),
            instance: value
                .get("instance_id")
                .and_then(Value::as_str)
                .map(str::to_string),
        }));
    }
    Ok(None)
}

pub(super) fn wait(pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !process::exists(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    !process::exists(pid)
}

pub(super) fn kill(pid: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        process::signal(pid, libc::SIGKILL)
    }

    #[cfg(windows)]
    {
        let status = Command::new("taskkill.exe")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|err| format!("taskkill failed: {err}"))?;
        if status.success() || !process::exists(pid) {
            Ok(())
        } else {
            Err(format!(
                "taskkill /PID {pid} /T /F exited with status {status}"
            ))
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        Err("force-kill is not implemented on this platform".to_string())
    }
}

pub(super) fn sanitize(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}
