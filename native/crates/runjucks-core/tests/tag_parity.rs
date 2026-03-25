//! JSON goldens from [`native/fixtures/conformance/tag_parity_cases.json`](../../../fixtures/conformance/tag_parity_cases.json).

use runjucks_core::loader::map_loader;
use runjucks_core::Environment;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct CaseEnv {
    #[serde(default, rename = "templateMap")]
    template_map: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    #[allow(dead_code)]
    source: Option<String>,
    template: String,
    #[serde(default)]
    context: Value,
    #[serde(default)]
    env: Option<CaseEnv>,
    expected: String,
    #[serde(default)]
    skip: bool,
}

#[test]
fn tag_parity_cases_match_expected() {
    let cases: Vec<Case> = serde_json::from_str(include_str!(
        "../../../fixtures/conformance/tag_parity_cases.json"
    ))
    .expect("parse tag_parity_cases.json");
    for case in cases {
        if case.skip {
            continue;
        }
        let mut env = Environment::default();
        if let Some(e) = &case.env {
            if let Some(map) = &e.template_map {
                env.loader = Some(map_loader(map.clone()));
            }
        }
        let out = env
            .render_string(case.template.clone(), case.context.clone())
            .unwrap_or_else(|e| panic!("case {}: {e}", case.id));
        assert_eq!(out, case.expected, "case {}", case.id);
    }
}
