use super::*;

pub(super) fn lease(target: &Target) -> Result<Option<u16>, String> {
    match target.port {
        Some(0) => {
            let listener = TcpListener::bind(("127.0.0.1", 0))
                .map_err(|err| format!("failed to lease a port for `{}`: {err}", target.name))?;
            let local = listener
                .local_addr()
                .map_err(|err| format!("failed to read the leased port: {err}"))?;
            Ok(Some(local.port()))
        }
        other => Ok(other),
    }
}

pub(super) fn health(
    target: &Target,
    state: &Map<String, Value>,
) -> Result<Option<String>, String> {
    let Some(template) = target.health.as_ref() else {
        return Ok(None);
    };
    if !template.contains('{') {
        return Ok(Some(template.clone()));
    }
    let held = state
        .get(&target.name)
        .and_then(|entry| entry.get("port"))
        .and_then(Value::as_u64);
    let Some(port) = held else {
        return Ok(None);
    };
    let vars = BTreeMap::from([("port", port.to_string())]);
    plumb::fill::fill(template, &vars)
        .map(Some)
        .map_err(|err| format!("`{}` health_url: {err}", target.name))
}

pub(super) fn purge(path: &Path, label: &str) -> Result<(), String> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_dir() => {
            fs::remove_dir_all(path)
                .map_err(|err| format!("failed to remove {label} dir {}: {err}", path.display()))?;
            println!("removed {label} dir {}", path.display());
            Ok(())
        }
        Ok(_) => Err(format!(
            "{label} path exists but is not a directory: {}",
            path.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "failed to inspect {label} dir {}: {err}",
            path.display()
        )),
    }
}

pub(super) fn pick<'plan>(
    plan: &'plan Plan,
    sidecar: Option<&str>,
) -> Result<Vec<&'plan Target>, String> {
    if let Some(name) = sidecar {
        let hit = plan
            .targets
            .iter()
            .find(|item| item.name == name)
            .ok_or_else(|| format!("unknown target `{name}` in this manifest"))?;
        Ok(vec![hit])
    } else {
        if plan.targets.is_empty() {
            return Err("manifest declares no lifecycle targets".to_string());
        }
        Ok(plan.targets.iter().collect())
    }
}
