//! Rust-only render microbenches (no NAPI). Mirrors `perf/synthetic.mjs` hot cases.
//!
//! Run: `cargo bench -p runjucks_core --bench render_hotspots`
//! Flamegraph (Linux, with `perf`): `cargo install flamegraph && cargo flamegraph --bench render_hotspots -p runjucks_core`

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use runjucks_core::Environment;
use serde_json::json;

fn for_medium(c: &mut Criterion) {
    let nums: Vec<u32> = (0..200).collect();
    let tpl = "{% for n in nums %}{{ n }}{% endfor %}".to_string();
    let env = Environment::default();
    c.bench_function("for_200_int_concat", |b| {
        b.iter(|| {
            let out = env
                .render_string(tpl.clone(), json!({ "nums": &nums }))
                .unwrap();
            black_box(out)
        })
    });
}

fn many_vars(c: &mut Criterion) {
    let mut ctx = serde_json::Map::new();
    let mut tpl = String::with_capacity(80 * 12);
    for i in 0..80 {
        tpl.push_str(&format!("{{{{ v{} }}}}", i));
        ctx.insert(
            format!("v{i}"),
            serde_json::Value::String(i.to_string()),
        );
    }
    let env = Environment::default();
    c.bench_function("many_vars_80", |b| {
        b.iter(|| {
            let out = env
                .render_string(tpl.clone(), serde_json::Value::Object(ctx.clone()))
                .unwrap();
            black_box(out)
        })
    });
}

fn nested_for(c: &mut Criterion) {
    let tpl = "{% for a in outer %}{% for b in a %}{{ b }}{% endfor %}{% endfor %}".to_string();
    let env = Environment::default();
    let ctx = json!({ "outer": [[1, 2], [3, 4]] });
    c.bench_function("nested_for_small", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

criterion_group!(benches, for_medium, many_vars, nested_for);
criterion_main!(benches);
