use std::collections::BTreeMap;

use crate::config;
use crate::config::Manifest;
use crate::stamp;
use crate::stamp::Stamp;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Plan {
    pub project: String,
    pub namespace: String,
    pub root: String,
    pub app: Option<App>,
    pub sidecars: Vec<Sidecar>,
    pub targets: Vec<Target>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct App {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub stamp: Stamp,
    pub env: BTreeMap<String, String>,
    pub inherits: Vec<Inherit>,
    pub inspect: bool,
    pub port: Option<u16>,
    pub health: Option<String>,
    pub ready: Option<Ready>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Sidecar {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub stamp: Stamp,
    pub env: BTreeMap<String, String>,
    pub inherits: Vec<Inherit>,
    pub inspect: bool,
    pub port: Option<u16>,
    pub health: Option<String>,
    pub ready: Option<Ready>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Target {
    pub name: String,
    pub kind: Kind,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub stamp: Stamp,
    pub env: BTreeMap<String, String>,
    pub inherits: Vec<Inherit>,
    pub inspect: bool,
    pub port: Option<u16>,
    pub health: Option<String>,
    pub ready: Option<Ready>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    App,
    Sidecar,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ready {
    pub role: String,
    pub timeout: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Inherit {
    pub name: String,
    pub from: String,
}

impl Manifest {
    pub fn plan(&self) -> Result<Plan, String> {
        let app = match &self.app {
            Some(app) => Some(app.plan(&self.project)?),
            None => None,
        };
        let mut sidecars = Vec::new();
        for sidecar in &self.sidecars {
            sidecars.push(sidecar.plan(&self.project)?);
        }
        let mut targets: Vec<Target> = sidecars.iter().map(Target::sidecar).collect();
        targets.extend(app.iter().map(Target::app));
        Ok(Plan {
            project: self.project.name.clone(),
            namespace: self.project.namespace.clone(),
            root: self.project.root.clone(),
            app,
            sidecars,
            targets,
        })
    }
}

impl Target {
    fn sidecar(plan: &Sidecar) -> Target {
        Target {
            name: plan.name.clone(),
            kind: Kind::Sidecar,
            command: plan.command.clone(),
            args: plan.args.clone(),
            cwd: plan.cwd.clone(),
            stamp: plan.stamp.clone(),
            env: plan.env.clone(),
            inherits: plan.inherits.clone(),
            inspect: plan.inspect,
            port: plan.port,
            health: plan.health.clone(),
            ready: plan.ready.clone(),
        }
    }

    fn app(plan: &App) -> Target {
        Target {
            name: plan.name.clone(),
            kind: Kind::App,
            command: plan.command.clone(),
            args: plan.args.clone(),
            cwd: plan.cwd.clone(),
            stamp: plan.stamp.clone(),
            env: plan.env.clone(),
            inherits: plan.inherits.clone(),
            inspect: plan.inspect,
            port: plan.port,
            health: plan.health.clone(),
            ready: plan.ready.clone(),
        }
    }
}

impl config::App {
    fn plan(&self, project: &config::Project) -> Result<App, String> {
        let stamp = Stamp {
            version: stamp::VERSION,
            app: self.name.clone(),
            namespace: project.namespace.clone(),
            mode: self.mode.clone(),
            source: stamp::default::SOURCE.to_string(),
        };
        Ok(App {
            name: self.name.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            cwd: self.cwd.clone(),
            stamp,
            env: self.env.clone(),
            inherits: self.inherits.iter().map(config::Inherit::plan).collect(),
            inspect: self.inspect.is_some(),
            port: self.port,
            health: self.health.clone(),
            ready: self.ready.as_ref().map(config::Ready::plan),
        })
    }
}

impl config::Sidecar {
    fn plan(&self, project: &config::Project) -> Result<Sidecar, String> {
        let stamp = Stamp {
            version: stamp::VERSION,
            app: self.name.clone(),
            namespace: project.namespace.clone(),
            mode: self.mode.clone(),
            source: stamp::default::SOURCE.to_string(),
        };
        Ok(Sidecar {
            name: self.name.clone(),
            command: self.command.clone(),
            args: self.args.clone(),
            cwd: self.cwd.clone(),
            stamp,
            env: self.env.clone(),
            inherits: self.inherits.iter().map(config::Inherit::plan).collect(),
            inspect: self.inspect.is_some(),
            port: self.port,
            health: self.health.clone(),
            ready: self.ready.as_ref().map(config::Ready::plan),
        })
    }
}

impl config::Ready {
    fn plan(&self) -> Ready {
        Ready {
            role: self.role.clone(),
            timeout: self.timeout,
        }
    }
}

impl config::Inherit {
    fn plan(&self) -> Inherit {
        Inherit {
            name: self.name.clone(),
            from: self.from.clone(),
        }
    }
}
