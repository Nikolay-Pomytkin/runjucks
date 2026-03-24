//! JSON-driven render conformance vs Nunjucks golden outputs.
//! Fixtures: [`native/fixtures/conformance/`](../../../fixtures/conformance/README.md)

use runjucks_core::Environment;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    #[allow(dead_code)]
    source: Option<String>,
    template: String,
    #[serde(default)]
    context: Value,
    env: Option<EnvOpt>,
    expected: String,
}

#[derive(Debug, Deserialize)]
struct EnvOpt {
    #[serde(default)]
    autoescape: Option<bool>,
    #[serde(default)]
    dev: Option<bool>,
}

fn all_cases() -> Vec<Case> {
    let mut out: Vec<Case> = serde_json::from_str(include_str!(
        "../../../fixtures/conformance/render_cases.json"
    ))
    .expect("parse render_cases.json");
    let filter: Vec<Case> = serde_json::from_str(include_str!(
        "../../../fixtures/conformance/filter_cases.json"
    ))
    .expect("parse filter_cases.json");
    out.extend(filter);
    out
}

#[test]
fn conformance_render_matches_nunjucks_golden_outputs() {
    for case in all_cases() {
        let mut env = Environment::default();
        if let Some(ref e) = case.env {
            if let Some(a) = e.autoescape {
                env.autoescape = a;
            }
            if let Some(d) = e.dev {
                env.dev = d;
            }
        }
        // Skip fixtures that need statement tags, filters, or `is` tests with call syntax.
        if case.template.contains("{%")
            || case.template.contains('|')
            || case.template.contains("equalto")
            || case.template.contains("sameas")
        {
            continue;
        }
        let result = env.render_string(case.template.clone(), case.context.clone());
        match result {
            Ok(out) => assert_eq!(out, case.expected, "case {}", case.id),
            Err(e) => panic!("case {}: render error: {} (source: {:?})", case.id, e, case.source),
        }
    }
}
