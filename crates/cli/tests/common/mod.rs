use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

pub fn stated(name: &str) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = fs::read_to_string(root.join(name)).unwrap_or_else(|err| panic!("{name}: {err}"));
    let bare: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect();
    serde_json::from_str(&bare.join("\n")).unwrap_or_else(|err| panic!("{name} is not json: {err}"))
}

pub fn schema() -> Map<String, Value> {
    stated("sidecar.schema.jsonc")["words"]
        .as_object()
        .cloned()
        .expect("schema words")
}
