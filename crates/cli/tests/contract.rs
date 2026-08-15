mod common;

use common::{schema, stated};
use serde_json::Value;
use std::collections::BTreeMap;

#[test]
fn conforming() {
    let schema = schema();
    let terms: BTreeMap<String, String> = schema
        .iter()
        .map(|(word, spec)| {
            (
                word.clone(),
                spec["term"].as_str().expect("term").to_string(),
            )
        })
        .collect();
    let fixture = stated("sidecar.fixture.jsonc");
    assert_eq!(
        fixture["schemaVersion"],
        stated("sidecar.schema.jsonc")["schemaVersion"],
        "the fixture and the schema must speak the same version"
    );
    let cases = fixture["cases"].as_array().expect("cases");
    assert!(!cases.is_empty(), "the fixture states no case");
    for case in cases {
        judge(case, &terms);
    }
}

fn judge(case: &Value, terms: &BTreeMap<String, String>) {
    let name = case["name"].as_str().unwrap_or_default();
    let held = case["environment"].as_object().expect("environment");
    let want = case["expects"].as_object().expect("expects");
    let mut keys: Vec<&String> = want.keys().collect();
    keys.sort();
    let mut all: Vec<&String> = terms.values().collect();
    all.sort();
    assert_eq!(
        keys, all,
        "case {name:?} must expect every term and no other"
    );
    for (term, value) in want {
        let word = terms
            .iter()
            .find(|(_, seat)| *seat == term)
            .map(|(word, _)| word)
            .expect("term maps to a word");
        assert_eq!(
            held.get(word).cloned().unwrap_or(Value::Null),
            *value,
            "case {name:?} expects {term} to follow {word}"
        );
    }
}

#[test]
fn tree() {
    let schema = schema();
    let terms: Vec<String> = schema
        .values()
        .map(|spec| spec["term"].as_str().expect("term").to_string())
        .collect();
    for one in &terms {
        let stem = format!("{one}_");
        let nested = terms.iter().find(|other| other.starts_with(&stem));
        assert!(
            nested.is_none(),
            "term {one:?} is a stem of {nested:?}; a binding that nests on term segments cannot render both"
        );
    }
}
