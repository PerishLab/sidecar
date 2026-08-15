use super::*;

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
