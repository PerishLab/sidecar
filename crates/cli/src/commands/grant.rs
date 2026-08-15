use super::*;

#[derive(Clone, Debug, Default)]
pub(crate) struct Grants {
    terms: BTreeMap<String, String>,
}

impl Grants {
    pub(super) fn lease(target: &Target, broker: &str, paths: &Paths) -> Result<Self, String> {
        let mut terms = BTreeMap::new();
        terms.insert("broker".to_string(), broker.to_string());
        if let Some(port) = port(target)? {
            terms.insert("port".to_string(), port.to_string());
        }
        if target.inspect {
            terms.insert(
                "inspect".to_string(),
                seat(target, paths).seat().to_string(),
            );
        }
        Ok(Self { terms })
    }

    pub(super) fn held(entry: &Value) -> Self {
        let mut terms = BTreeMap::new();
        if let Some(fields) = entry.get("grants").and_then(Value::as_object) {
            for (term, value) in fields {
                if let Some(text) = value.as_str() {
                    terms.insert(term.clone(), text.to_string());
                }
            }
        }
        Self { terms }
    }

    pub(super) fn announce(&self, command: &mut Command) {
        for (term, value) in &self.terms {
            command.env(word(term), value);
        }
    }

    pub(super) fn fill(&self, template: &str, name: &str) -> Result<String, String> {
        let vars: BTreeMap<&str, String> = self
            .terms
            .iter()
            .map(|(term, value)| (term.as_str(), value.clone()))
            .collect();
        plumb::fill::fill(template, &vars).map_err(|err| format!("`{name}`: {err}"))
    }

    pub(super) fn record(&self) -> Value {
        Value::Object(
            self.terms
                .iter()
                .map(|(term, value)| (term.clone(), Value::String(value.clone())))
                .collect(),
        )
    }

    pub(super) fn holds(&self, term: &str) -> bool {
        self.terms.contains_key(term)
    }
}

pub(crate) fn seat(target: &Target, paths: &Paths) -> bridge::Bridge {
    bridge::Bridge::new(&paths.project, &target.stamp.namespace, &target.name)
}

pub(crate) fn word(term: &str) -> String {
    format!("SIDECAR_{}", term.to_uppercase())
}

fn port(target: &Target) -> Result<Option<u16>, String> {
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
    let Some(entry) = state.get(&target.name) else {
        return Ok(None);
    };
    let grants = Grants::held(entry);
    if !grants.holds("port") {
        return Ok(None);
    }
    grants
        .fill(template, &format!("{} health_url", target.name))
        .map(Some)
}
