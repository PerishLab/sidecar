pub(crate) mod broker;
mod grant;
mod host;
mod logs;
mod pick;
mod ready;
mod render;
mod runtime;
mod wait;
use grant::{Grants, health};
pub(crate) use host::host;
use pick::{pick, purge};

use crate::args::Waiting;
use crate::cli::Format;
use ready::Chain;
use runtime::{Broker, Launch};
use serde_json::{Map, Value};
use sidecar_core::plan::{Plan, Target};
use sidecar_core::{Paths, State, bridge, inspect, process};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub(crate) struct Session {
    pub(crate) state: State,
    pub(crate) paths: Paths,
}

pub(crate) struct Probe<'a> {
    pub(crate) sidecar: &'a str,
    pub(crate) event: &'a str,
    pub(crate) payload: Option<&'a str>,
    pub(crate) timeout: Duration,
}

impl Session {
    pub(crate) fn start(&self, sidecar: Option<&str>, waiting: Waiting) -> Result<(), String> {
        let plan = self.state.plan()?;
        let targets = pick(&plan, sidecar)?;
        let endpoint = Broker::new(&plan).ensure()?;
        let mut chain = Chain::load(&self.paths, &plan)?;
        for target in targets {
            if let Some(running) = runtime::running(&self.paths, target)?.first() {
                return Err(format!(
                    "sidecar `{}` is already running (pid {}); run `sidecar stop` first",
                    target.name, running
                ));
            }
            let env = chain.inherits(target)?;
            let launch = self.spawn(target, &endpoint, &env)?;
            runtime::state::record(&self.paths, target, &launch)?;
            if let Some(ready) = &launch.ready {
                chain.record(&target.name, ready);
            }
            if waiting.wait {
                self.settle(target, &launch, waiting.timeout)?;
            }
            println!("started {} pid={}", target.name, launch.pid);
        }
        Ok(())
    }

    pub(crate) fn stop(&self, sidecar: Option<&str>, force: bool) -> Result<(), String> {
        let plan = self.state.plan()?;
        let targets = pick(&plan, sidecar)?;
        let mut stopped = 0;
        for target in targets {
            let pids = runtime::running(&self.paths, target)?;
            if pids.is_empty() {
                println!("not running: {}", target.name);
                continue;
            }
            for pid in &pids {
                process::stop(*pid).map_err(|err| {
                    format!(
                        "failed to terminate sidecar `{}` (pid {}): {err}",
                        target.name, pid
                    )
                })?;
                runtime::reap(*pid, force)?;
                println!("stopped {} pid={}", target.name, pid);
                stopped += 1;
            }
            runtime::state::remove(&self.paths, &target.name)?;
        }
        if stopped == 0 && sidecar.is_none() {
            println!("no sidecars were running");
        }
        let broker = Broker::new(&plan);
        if sidecar.is_none() || broker.idle(&self.paths)? {
            broker.stop(force)?;
        }
        Ok(())
    }

    pub(crate) fn restart(
        &self,
        sidecar: Option<&str>,
        force: bool,
        waiting: Waiting,
    ) -> Result<(), String> {
        self.stop(sidecar, force)?;
        self.start(sidecar, waiting)
    }

    fn settle(&self, target: &Target, launch: &Launch, patience: u64) -> Result<(), String> {
        let Err(err) = wait::wait(target, &launch.grants, patience) else {
            return Ok(());
        };
        let _ = process::stop(launch.host);
        let _ = runtime::reap(launch.host, true);
        let _ = runtime::state::remove(&self.paths, &target.name);
        Err(err)
    }

    pub(crate) fn status(&self, format: Format) -> Result<(), String> {
        let plan = self.state.plan()?;
        let state = runtime::state::load(&self.paths)?;
        let mut rows = Vec::new();
        for target in &plan.targets {
            let pids = runtime::running(&self.paths, target)?;
            rows.push(render::Row {
                name: target.name.clone(),
                pids,
                target: runtime::state::seat(&state, &target.name),
                log: runtime::state::log(&self.paths, &target.name),
                health: health(target, &state)?,
            });
        }
        let broker = Broker::new(&plan).status()?;
        render::status(&plan.namespace, &rows, &broker, format)
    }

    pub(crate) fn logs(
        &self,
        sidecar: Option<&str>,
        follow: bool,
        lines: Option<usize>,
    ) -> Result<(), String> {
        let plan = self.state.plan()?;
        logs::logs(&self.paths, &pick(&plan, sidecar)?, follow, lines)
    }

    pub(crate) fn list(&self, format: Format) -> Result<(), String> {
        let plan = self.state.plan()?;
        let hits = process::Stamped::discover(None, &plan.namespace)
            .map_err(|err| format!("discovery failed for namespace `{}`: {err}", plan.namespace))?;
        let broker = Broker::new(&plan).status()?;
        let state = runtime::state::load(&self.paths)?;
        let listing = render::Listing {
            namespace: &plan.namespace,
            hits: &hits,
            broker: &broker,
            state: &state,
        };
        render::list(&listing, format)
    }

    pub(crate) fn reset(&self, all: bool, force: bool) -> Result<(), String> {
        let plan = self.state.plan()?;
        for target in &plan.targets {
            for pid in runtime::running(&self.paths, target)? {
                process::stop(pid)
                    .map_err(|err| format!("failed to terminate pid {pid}: {err}"))?;
                runtime::reap(pid, force)?;
                println!("terminated pid={pid} target={}", target.name);
            }
        }
        let hits = process::Stamped::discover(None, &plan.namespace)
            .map_err(|err| format!("discovery failed for namespace `{}`: {err}", plan.namespace))?;
        if hits.is_empty() {
            println!("namespace `{}` has no stamped processes", plan.namespace);
        } else {
            for hit in &hits {
                process::stop(hit.pid)
                    .map_err(|err| format!("failed to terminate pid {}: {err}", hit.pid))?;
                runtime::reap(hit.pid, force)?;
                println!("terminated pid={} cmd={}", hit.pid, hit.command);
            }
        }
        Broker::new(&plan).stop(force)?;
        purge(&self.paths.project, "project data")?;
        if all {
            purge(&self.paths.state, "global state")?;
        }
        Ok(())
    }

    pub(crate) fn inspect(&self, probe: &Probe, format: Format) -> Result<(), String> {
        let plan = self.state.plan()?;
        let target = plan
            .targets
            .iter()
            .find(|item| item.name == probe.sidecar)
            .ok_or_else(|| format!("unknown target `{}` in this manifest", probe.sidecar))?;
        if !target.inspect {
            return Err(format!(
                "target `{}` declares no [inspect] section in this manifest",
                probe.sidecar
            ));
        }
        let bridge = grant::seat(target, &self.paths);
        let body: Value = match probe.payload {
            Some(text) if !text.is_empty() => serde_json::from_str(text).map_err(|err| {
                format!("payload is not valid JSON: {err}; quote the payload as a single argument")
            })?,
            _ => serde_json::json!({}),
        };
        let request = inspect::Request {
            event: probe.event.to_string(),
            payload: body,
        };
        let response = inspect::send(&bridge, &request, Some(probe.timeout))?;
        render::inspect(probe.sidecar, probe.event, &response, format)
    }

    fn spawn(
        &self,
        target: &Target,
        endpoint: &str,
        env: &[(String, String)],
    ) -> Result<Launch, String> {
        let cwd = self.cwd(&target.cwd);
        let path = runtime::state::log(&self.paths, &target.name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
        if target.inspect {
            let seat = self.paths.project.join("inspect");
            fs::create_dir_all(&seat)
                .map_err(|err| format!("failed to create {}: {err}", seat.display()))?;
        }
        let exe = std::env::current_exe()
            .map_err(|err| format!("failed to resolve the sidecar binary: {err}"))?;
        let mut command = Command::new(exe);
        command
            .args(["runtime", "host"])
            .args(target.stamp.args())
            .arg("--")
            .arg(&target.command)
            .args(&target.args)
            .current_dir(&cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(file));
        let grants = Grants::lease(target, endpoint, &self.paths)?;
        for (key, value) in &target.env {
            let held = grants.fill(value, &format!("{} env {key}", target.name))?;
            command.env(key, held);
        }
        for (key, value) in env {
            command.env(key, value);
        }
        grants.announce(&mut command);
        runtime::detach(&mut command);
        let mut child = command
            .spawn()
            .map_err(|err| format!("failed to host `{}`: {err}", target.name))?;
        let host = child.id();
        let pid = host::greet(&mut child, &target.name)?;
        let ready = match &target.ready {
            Some(ready) => Some(runtime::watch(
                &mut child,
                &path,
                &ready.role,
                ready.timeout,
            )?),
            None => None,
        };
        Ok(Launch {
            host,
            pid,
            ready,
            log: path,
            grants,
        })
    }

    fn cwd(&self, cwd: &str) -> PathBuf {
        let path = Path::new(cwd);
        if path.is_absolute() {
            return path.to_path_buf();
        }
        match self.state.path.parent() {
            Some(dir) => dir.join(path),
            None => path.to_path_buf(),
        }
    }
}
