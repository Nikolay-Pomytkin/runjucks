//! Rust-only render microbenches (no NAPI). Mirrors `perf/synthetic.mjs` hot cases.
//!
//! Run from repo root: `cargo bench --manifest-path native/Cargo.toml -p runjucks_core --bench render_hotspots`
//! Or from `native/`: `cargo bench -p runjucks_core --bench render_hotspots`
//! Flamegraph (Linux, with `perf`): `cargo install flamegraph && cd native && cargo flamegraph --bench render_hotspots -p runjucks_core`

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

fn for_200_binary_add_context(c: &mut Criterion) {
    let nums: Vec<u32> = (0..200).collect();
    let tpl = "{% for n in nums %}{{ n + k }}{% endfor %}".to_string();
    let env = Environment::default();
    let ctx = json!({ "nums": &nums, "k": 3 });
    c.bench_function("for_200_var_plus_context_int", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn for_200_with_loop_index(c: &mut Criterion) {
    let nums: Vec<u32> = (0..200).collect();
    let tpl = "{% for n in nums %}{{ loop.index }}:{{ n }}{% endfor %}".to_string();
    let env = Environment::default();
    c.bench_function("for_200_loop_index_and_item", |b| {
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
        ctx.insert(format!("v{i}"), serde_json::Value::String(i.to_string()));
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

fn literal_string_upper_filter(c: &mut Criterion) {
    let tpl = "{{ 'hello' | upper }}".to_string();
    let env = Environment::default();
    c.bench_function("literal_string_upper_filter", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), json!({})).unwrap();
            black_box(out)
        })
    });
}

fn attr_chain_three(c: &mut Criterion) {
    let tpl = "{{ u.a.b.c }}".to_string();
    let env = Environment::default();
    let ctx = json!({ "u": { "a": { "b": { "c": "x" } } } });
    c.bench_function("attr_chain_three_depth", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn literal_length_filter(c: &mut Criterion) {
    let tpl = "{{ 'hello' | length }}".to_string();
    let env = Environment::default();
    c.bench_function("literal_string_length_filter", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), json!({})).unwrap();
            black_box(out)
        })
    });
}

fn variable_chained_upper_lower(c: &mut Criterion) {
    let tpl = "{{ s | upper | lower }}".to_string();
    let env = Environment::default();
    let ctx = json!({ "s": "Hello World" });
    c.bench_function("variable_chained_upper_lower_filters", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn variable_trim_then_upper(c: &mut Criterion) {
    let tpl = "{{ s | trim | upper }}".to_string();
    let env = Environment::default();
    let ctx = json!({ "s": "  hello  " });
    c.bench_function("variable_trim_upper_filters", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn variable_trim_capitalize_chain(c: &mut Criterion) {
    let tpl = "{{ s | trim | capitalize }}".to_string();
    let env = Environment::default();
    let ctx = json!({ "s": "  hELLO  " });
    c.bench_function("variable_trim_capitalize_filters", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn variable_lower_title_chain(c: &mut Criterion) {
    let tpl = "{{ s | lower | title }}".to_string();
    let env = Environment::default();
    let ctx = json!({ "s": "hello WORLD" });
    c.bench_function("variable_lower_title_filters", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

/// `joiner()` handle invoked via `Expr::Call` — exercises `resolve_variable_ref` + `parse_joiner_id`.
fn joiner_three_calls(c: &mut Criterion) {
    let tpl = "{% set j = joiner(',') %}{{ j() }}a{{ j() }}b{{ j() }}c".to_string();
    let env = Environment::default();
    let ctx = json!({});
    c.bench_function("joiner_set_and_three_calls", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn conditional_macro_iter_switch_filters(c: &mut Criterion) {
    let tpl = "{% macro fmt(v) -%}{% switch v.kind %}{% case 'warn' %}{{ v.msg | trim | upper }}{% case 'ok' %}{{ v.msg | trim | lower }}{% default %}{{ v.msg | trim }}{% endswitch %}{%- endmacro %}{% for v in rows %}{% if v.enabled %}{{ fmt(v) }}{% endif %}{% endfor %}".to_string();
    let rows: Vec<_> = (0..80)
        .map(|i| {
            json!({
                "enabled": i % 3 != 0,
                "kind": if i % 2 == 0 { "warn" } else { "ok" },
                "msg": format!("  Row {}  ", i)
            })
        })
        .collect();
    let env = Environment::default();
    let ctx = json!({ "rows": rows });
    c.bench_function("conditional_macro_iter_switch_filters", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn switch_in_for_attr_filters(c: &mut Criterion) {
    let tpl = "{% for row in rows %}{% switch row.type %}{% case 'a' %}{{ row.payload.title | trim | upper }}{% case 'b' %}{{ row.payload.title | trim | lower }}{% default %}{{ row.payload.title | trim }}{% endswitch %}{% endfor %}".to_string();
    let rows: Vec<_> = (0..120)
        .map(|i| {
            json!({
                "type": if i % 3 == 0 { "a" } else if i % 3 == 1 { "b" } else { "c" },
                "payload": { "title": format!("  T{}  ", i) }
            })
        })
        .collect();
    let env = Environment::default();
    let ctx = json!({ "rows": rows });
    c.bench_function("switch_in_for_with_attr_filters", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn macro_call_with_filter_chain_in_loop(c: &mut Criterion) {
    let tpl = "{% macro fmt(v) -%}{{ v.msg | trim | upper }}|{{ v.alt | trim | lower }}{%- endmacro %}{% for v in rows %}{{ fmt(v) }}{% endfor %}".to_string();
    let rows: Vec<_> = (0..100)
        .map(|i| {
            json!({
                "msg": format!("  Row {}  ", i),
                "alt": format!("  ALT {}  ", i)
            })
        })
        .collect();
    let env = Environment::default();
    let ctx = json!({ "rows": rows });
    c.bench_function("macro_call_with_filter_chain_in_loop", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn inline_if_filter_chain_dense(c: &mut Criterion) {
    let tpl = "{% for row in rows %}{{ (row.msg | trim | upper) if row.enabled else (row.msg | trim | lower) }}{% endfor %}".to_string();
    let rows: Vec<_> = (0..120)
        .map(|i| {
            json!({
                "enabled": i % 2 == 0,
                "msg": format!("  Dense {}  ", i)
            })
        })
        .collect();
    let env = Environment::default();
    let ctx = json!({ "rows": rows });
    c.bench_function("inline_if_filter_chain_dense", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

fn call_block_with_args_in_loop(c: &mut Criterion) {
    let tpl = "{% macro wrap(items) -%}{% for item in items %}[{{ caller(item) }}]{% endfor %}{%- endmacro %}{% call(row, suffix='!') wrap(rows) %}{{ row.msg | trim | upper }}{{ suffix }}{% endcall %}".to_string();
    let rows: Vec<_> = (0..100)
        .map(|i| {
            json!({
                "msg": format!("  Call {}  ", i)
            })
        })
        .collect();
    let env = Environment::default();
    let ctx = json!({ "rows": rows });
    c.bench_function("call_block_with_args_in_loop", |b| {
        b.iter(|| {
            let out = env.render_string(tpl.clone(), ctx.clone()).unwrap();
            black_box(out)
        })
    });
}

criterion_group!(
    benches,
    for_medium,
    for_200_binary_add_context,
    for_200_with_loop_index,
    many_vars,
    nested_for,
    literal_string_upper_filter,
    attr_chain_three,
    literal_length_filter,
    variable_chained_upper_lower,
    variable_trim_then_upper,
    variable_trim_capitalize_chain,
    variable_lower_title_chain,
    joiner_three_calls,
    conditional_macro_iter_switch_filters,
    switch_in_for_attr_filters,
    macro_call_with_filter_chain_in_loop,
    inline_if_filter_chain_dense,
    call_block_with_args_in_loop
);
criterion_main!(benches);
