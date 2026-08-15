use crate::config::Manifest;
use crate::diagnostics::Diagnostic;
use crate::plan::Plan;
use std::collections::HashSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct State {
    pub path: PathBuf,
    pub config: Manifest,
}

#[derive(Debug)]
pub enum Error {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
}

impl State {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let text = fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        let config = toml::from_str(&text).map_err(|source| Error::Parse {
            path: path.clone(),
            source: Box::new(source),
        })?;
        Ok(Self { path, config })
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        require(&mut diagnostics, "project.name", &self.config.project.name);
        require(
            &mut diagnostics,
            "project.namespace",
            &self.config.project.namespace,
        );

        if let Some(app) = &self.config.app {
            require(&mut diagnostics, "app.name", &app.name);
            require(&mut diagnostics, "app.command", &app.command);
            require(&mut diagnostics, "app.mode", &app.mode);
            if let Some(ready) = &app.ready {
                require(&mut diagnostics, "app.ready.role", &ready.role);
            }
        } else if self.config.sidecars.is_empty() {
            diagnostics.push(Diagnostic::warning(
                "app",
                "no app or sidecar command is configured; lifecycle commands have no targets",
            ));
        }

        let mut names = HashSet::new();
        for (index, sidecar) in self.config.sidecars.iter().enumerate() {
            let path = format!("sidecars[{index}]");
            require(&mut diagnostics, format!("{path}.name"), &sidecar.name);
            require(
                &mut diagnostics,
                format!("{path}.command"),
                &sidecar.command,
            );
            require(&mut diagnostics, format!("{path}.mode"), &sidecar.mode);
            if !sidecar.name.trim().is_empty() && !names.insert(sidecar.name.as_str()) {
                diagnostics.push(Diagnostic::error(
                    format!("{path}.name"),
                    format!("duplicate sidecar name `{}`", sidecar.name),
                ));
            }
            if let Some(ready) = &sidecar.ready {
                require(&mut diagnostics, format!("{path}.ready.role"), &ready.role);
            }
        }

        diagnostics
    }

    pub fn plan(&self) -> Result<Plan, String> {
        self.config.plan()
    }
}

fn require(diagnostics: &mut Vec<Diagnostic>, path: impl Into<String>, value: &str) {
    if value.trim().is_empty() {
        diagnostics.push(Diagnostic::error(path, "value must not be empty"));
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read { path, source } => {
                write!(formatter, "failed to read {}: {source}", path.display())
            }
            Error::Parse { path, source } => {
                write!(formatter, "failed to parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for Error {}
