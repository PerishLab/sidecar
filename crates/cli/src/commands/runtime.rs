use super::grant::Grants;
use super::ready::{Ready, kill, sanitize, scan, wait};
use serde_json::Value;
use sidecar_core::plan::{Plan, Target};
use sidecar_core::{Paths, broker, process};
use std::fs::OpenOptions;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Status {
    pub(crate) pids: Vec<u32>,
    pub(crate) endpoint: Option<String>,
}

#[derive(Clone, Debug)]

pub(crate) struct Launch {
    pub(crate) host: u32,
    pub(crate) pid: u32,
    pub(crate) ready: Option<Ready>,
    pub(crate) log: std::path::PathBuf,
    pub(crate) grants: Grants,
}

pub(crate) struct Broker<'a> {
    plan: &'a Plan,
}

impl<'a> Broker<'a> {
    pub(crate) fn new(plan: &'a Plan) -> Self {
        Self { plan }
    }

    fn identity(&self) -> broker::Identity {
        broker::Identity::new(&self.plan.project, &self.plan.namespace)
    }

    pub(crate) fn ensure(&self) -> Result<String, String> {
        let identity = self.identity();
        if let Some(addr) = identity.endpoint(Duration::from_millis(200))? {
            return Ok(format!("tcp://{addr}"));
        }

        let exe = std::env::current_exe()
            .map_err(|err| format!("failed to resolve sidecar exe: {err}"))?;
        let path = std::env::temp_dir().join(format!(
            "sidecar-broker-{}-{}.log",
            sanitize(&identity.project),
            sanitize(&identity.namespace)
        ));
        let log = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
        let stderr = log
            .try_clone()
            .map_err(|err| format!("failed to clone {}: {err}", path.display()))?;
        let mut command = Command::new(exe);
        command
            .args(["runtime", "serve", &identity.project, &identity.namespace])
            .args(identity.args())
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr));
        detach(&mut command);
        let child = command
            .spawn()
            .map_err(|err| format!("failed to spawn broker: {err}"))?;
        let pid = child.id();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(addr) = identity.endpoint(Duration::from_millis(200))? {
                println!("broker runtime pid={pid} endpoint=tcp://{addr}");
                return Ok(format!("tcp://{addr}"));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(format!(
            "timed out waiting for broker runtime pid={pid}; see {}",
            path.display()
        ))
    }

    pub(crate) fn status(&self) -> Result<Status, String> {
        let identity = self.identity();
        let brokers = process::Broker::discover(&identity.project, &identity.namespace)?;
        let endpoint = identity.endpoint(Duration::from_millis(200))?;
        Ok(Status {
            pids: brokers.into_iter().map(|broker| broker.pid).collect(),
            endpoint: endpoint.map(|addr| format!("tcp://{addr}")),
        })
    }

    pub(crate) fn stop(&self, force: bool) -> Result<(), String> {
        let identity = self.identity();
        let brokers = process::Broker::discover(&identity.project, &identity.namespace)?;
        for hit in brokers {
            process::stop(hit.pid)
                .map_err(|err| format!("failed to terminate broker pid {}: {err}", hit.pid))?;
            reap(hit.pid, force)?;
            println!("stopped broker pid={}", hit.pid);
        }
        Ok(())
    }

    pub(crate) fn idle(&self, paths: &Paths) -> Result<bool, String> {
        for target in &self.plan.targets {
            if !running(paths, target)?.is_empty() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

pub(crate) fn watch(
    child: &mut Child,
    log: &Path,
    role: &str,
    timeout: u64,
) -> Result<Ready, String> {
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("failed to check child status: {err}"))?
        {
            return Err(format!(
                "target exited before ready with status {status}; see {}",
                log.display()
            ));
        }
        if let Some(ready) = scan(log, role)? {
            return Ok(ready);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Err(format!(
        "timed out waiting for ready role {role:?}; see {}",
        log.display()
    ))
}

pub(crate) mod state {
    use super::Launch;
    use serde_json::{Map, Value};
    use sidecar_core::Paths;
    use sidecar_core::plan::Target;
    use std::fs;
    use std::path::PathBuf;

    fn path(paths: &Paths) -> PathBuf {
        paths.project.join("targets.json")
    }

    pub(crate) fn load(paths: &Paths) -> Result<Map<String, Value>, String> {
        let path = path(paths);
        let Ok(text) = fs::read_to_string(&path) else {
            return Ok(Map::new());
        };
        let value: Value = serde_json::from_str(&text)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
        Ok(value.as_object().cloned().unwrap_or_default())
    }

    fn save(paths: &Paths, state: &Map<String, Value>) -> Result<(), String> {
        fs::create_dir_all(&paths.project)
            .map_err(|err| format!("failed to create {}: {err}", paths.project.display()))?;
        let path = path(paths);
        let text = serde_json::to_string_pretty(state).map_err(|err| err.to_string())?;
        fs::write(&path, text).map_err(|err| format!("failed to write {}: {err}", path.display()))
    }

    pub(crate) fn record(paths: &Paths, target: &Target, launch: &Launch) -> Result<(), String> {
        let mut state = load(paths)?;
        state.insert(
            target.name.clone(),
            serde_json::json!({
                "pid": launch.host,
                "target": launch.pid,
                "app": target.stamp.app,
                "namespace": target.stamp.namespace,
                "mode": target.stamp.mode,
                "source": target.stamp.source,
                "grants": launch.grants.record(),
                "logPath": launch.log.display().to_string(),
                "ready": launch.ready.as_ref().map(|ready| serde_json::json!({
                    "role": ready.role,
                    "endpoint": ready.endpoint,
                    "runtimeEndpoint": ready.runtime,
                    "instanceId": ready.instance,
                })),
            }),
        );
        save(paths, &state)
    }

    pub(crate) fn remove(paths: &Paths, name: &str) -> Result<(), String> {
        let mut state = load(paths)?;
        state.remove(name);
        save(paths, &state)
    }
}

pub(crate) fn running(paths: &Paths, target: &Target) -> Result<Vec<u32>, String> {
    let mut pids = Vec::new();
    if let Some(pid) = state::load(paths)?
        .get(&target.name)
        .and_then(|entry| entry.get("pid"))
        .and_then(Value::as_u64)
        .and_then(|pid| u32::try_from(pid).ok())
        .filter(|pid| process::exists(*pid))
    {
        pids.push(pid);
    }
    let hits = process::Stamped::discover(Some(&target.stamp.app), &target.stamp.namespace)
        .map_err(|err| format!("discovery failed for `{}`: {err}", target.name))?;
    for hit in hits {
        if !pids.contains(&hit.pid) {
            pids.push(hit.pid);
        }
    }
    Ok(pids)
}

pub(crate) fn reap(pid: u32, force: bool) -> Result<(), String> {
    if wait(pid, Duration::from_secs(2)) {
        return Ok(());
    }
    if !force {
        return Err(format!(
            "pid {pid} did not exit after graceful stop; rerun with --force to kill it"
        ));
    }
    kill(pid)?;
    if wait(pid, Duration::from_secs(2)) {
        return Ok(());
    }
    Err(format!("timed out waiting for pid {pid} to exit"))
}

pub(crate) fn detach(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = command;
    }
}
