use crate::config::{self, Manifest};
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
    Section {
        path: PathBuf,
        why: String,
    },
}

impl State {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref().to_path_buf();
        let config = if embedded(&path) {
            section(&path)?
        } else {
            whole(&path)?
        };
        Ok(Self { path, config })
    }

    pub fn probe(path: impl AsRef<Path>) -> Result<bool, Error> {
        let path = path.as_ref();
        match radicula_config::section::<toml::Table>(path, config::SECTION) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(error) => Err(convert(path.to_path_buf(), error)),
        }
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

fn embedded(path: &Path) -> bool {
    path.file_name().is_some_and(|name| {
        if cfg!(windows) {
            name.to_str()
                .is_some_and(|value| value.eq_ignore_ascii_case(config::PLUMB))
        } else {
            name == config::PLUMB
        }
    })
}

fn whole(path: &Path) -> Result<Manifest, Error> {
    let text = fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| Error::Parse {
        path: path.to_path_buf(),
        source: Box::new(source),
    })
}

fn section(path: &Path) -> Result<Manifest, Error> {
    fs::metadata(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    match radicula_config::section(path, config::SECTION) {
        Ok(Some(config)) => Ok(config),
        Ok(None) => Err(Error::Section {
            path: path.to_path_buf(),
            why: format!("missing [{}] section", config::SECTION),
        }),
        Err(error) => Err(convert(path.to_path_buf(), error)),
    }
}

fn convert(path: PathBuf, error: radicula_config::Error) -> Error {
    match error {
        radicula_config::Error::Read { path, source } => Error::Read { path, source },
        radicula_config::Error::Parse { path, source } => Error::Parse {
            path,
            source: Box::new(source),
        },
        radicula_config::Error::Section { path, why, .. } => Error::Section { path, why },
        radicula_config::Error::Env { why, .. } => Error::Section { path, why },
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
            Error::Section { path, why } => {
                write!(
                    formatter,
                    "failed to load [{}] from {}: {why}",
                    config::SECTION,
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for Error {}
