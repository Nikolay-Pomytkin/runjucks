//! Guardrails for P1 perf work: `Node::Text` (`Arc<str>`), `CtxStack` (`ahash` maps + `Arc<Value>`
//! per binding), unary `resolve_variable_ref` fast paths, literal filter shortcuts, dotted
//! attribute chains (`foo.bar.baz`), and member/index chains on plain variables without cloning the
//! whole container when possible.
//! Documented in `runjucks/ai_docs/RUNJUCKS_PERF.md` (changelog + P0 regression list).

use runjucks_core::value::value_to_string;
use runjucks_core::Environment;
use serde_json::{json, Value};
use std::sync::Arc;

#[test]
fn literal_string_upper_lower_unicode_matches_expectation() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ 'café' | upper }}".into(), json!({}))
            .unwrap(),
        "CAFÉ"
    );
    assert_eq!(
        env.render_string("{{ 'CAFÉ' | lower }}".into(), json!({}))
            .unwrap(),
        "café"
    );
}

#[test]
fn literal_upper_fast_path_skipped_when_custom_filter_registered() {
    let mut env = Environment::default();
    env.add_filter(
        "upper",
        Arc::new(|input, _args| Ok(Value::String(format!("custom:{}", value_to_string(input))))),
    );
    let out = env
        .render_string("{{ 'hello' | upper }}".into(), json!({}))
        .unwrap();
    assert_eq!(out, "custom:hello");
}

#[test]
fn chained_filters_on_literal_still_apply_in_order() {
    let env = Environment::default();
    let out = env
        .render_string("{{ 'aBc' | upper | lower }}".into(), json!({}))
        .unwrap();
    assert_eq!(out, "abc");
}

#[test]
fn chained_upper_lower_on_variable_matches_sequential_filters() {
    let env = Environment::default();
    let ctx = json!({ "s": "aBc" });
    assert_eq!(
        env.render_string("{{ s | upper | lower }}".into(), ctx.clone())
            .unwrap(),
        "abc"
    );
}

#[test]
fn variable_upper_then_length_counts_uppercased_string() {
    let env = Environment::default();
    let ctx = json!({ "s": "aBc" });
    assert_eq!(
        env.render_string("{{ s | upper | length }}".into(), ctx)
            .unwrap(),
        "3"
    );
}

#[test]
fn fused_filter_chain_trim_upper_variable() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ s | trim | upper }}".into(), json!({ "s": "  hello  " }),)
            .unwrap(),
        "HELLO"
    );
    assert_eq!(
        env.render_string("{{ s | upper | trim }}".into(), json!({ "s": "  hello  " }),)
            .unwrap(),
        "HELLO"
    );
}

#[test]
fn fused_filter_chain_trim_capitalize_variable() {
    let env = Environment::default();
    assert_eq!(
        env.render_string(
            "{{ s | trim | capitalize }}".into(),
            json!({ "s": "  hELLO  " }),
        )
        .unwrap(),
        "Hello"
    );
}

#[test]
fn capitalize_fusion_skipped_when_custom_capitalize_registered() {
    let mut env = Environment::default();
    env.add_filter(
        "capitalize",
        Arc::new(|input, _args| Ok(Value::String(format!("custom:{}", value_to_string(input))))),
    );
    let out = env
        .render_string("{{ 'ab' | capitalize }}".into(), json!({}))
        .unwrap();
    assert_eq!(out, "custom:ab");
}

#[test]
fn single_filter_capitalize_fast_path_literal_and_variable() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ 'heLLo' | capitalize }}".into(), json!({}))
            .unwrap(),
        "Hello"
    );
    assert_eq!(
        env.render_string("{{ s | capitalize }}".into(), json!({ "s": "heLLo" }))
            .unwrap(),
        "Hello"
    );
}

#[test]
fn fused_compare_single_step_avoids_redundant_clone_smoke() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ a == b }}".into(), json!({ "a": 1, "b": 1 }))
            .unwrap(),
        "true"
    );
    assert_eq!(
        env.render_string("{{ a == b }}".into(), json!({ "a": 1, "b": 2 }))
            .unwrap(),
        "false"
    );
}

#[test]
fn trim_fusion_skipped_when_custom_trim_registered() {
    let mut env = Environment::default();
    env.add_filter(
        "trim",
        Arc::new(|input, _args| Ok(Value::String(format!("custom:{}", value_to_string(input))))),
    );
    let out = env
        .render_string("{{ 'x' | trim }}".into(), json!({}))
        .unwrap();
    assert_eq!(out, "custom:x");
}

#[test]
fn is_divisibleby_borrows_variable_lhs_with_arg() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ v is divisibleby(2) }}".into(), json!({ "v": 8 }))
            .unwrap(),
        "true"
    );
    assert_eq!(
        env.render_string("{{ v is divisibleby(3) }}".into(), json!({ "v": 8 }))
            .unwrap(),
        "false"
    );
}

#[test]
fn get_attr_on_plain_variable_reads_one_field() {
    let env = Environment::default();
    let ctx = json!({ "user": { "name": "Ada", "noise": "ignore" } });
    let out = env.render_string("{{ user.name }}".into(), ctx).unwrap();
    assert_eq!(out, "Ada");
}

#[test]
fn nested_three_level_getattr_chain_reads_leaf() {
    let env = Environment::default();
    let ctx = json!({ "u": { "a": { "b": { "c": "leaf" } } } });
    let out = env.render_string("{{ u.a.b.c }}".into(), ctx).unwrap();
    assert_eq!(out, "leaf");
}

#[test]
fn get_item_literal_index_and_key_on_variable_base() {
    let env = Environment::default();
    let ctx = json!({
        "arr": [10, 20],
        "obj": { "k": "v" }
    });
    assert_eq!(
        env.render_string("{{ arr[1] }}".into(), ctx.clone())
            .unwrap(),
        "20"
    );
    assert_eq!(
        env.render_string("{{ obj['k'] }}".into(), ctx).unwrap(),
        "v"
    );
}

#[test]
fn slice_on_variable_base_matches_expectation() {
    let env = Environment::default();
    let out = env
        .render_string(
            "{{ arr[0:2] | join(',') }}".into(),
            json!({ "arr": [1, 2, 3] }),
        )
        .unwrap();
    assert_eq!(out, "1,2");
}

#[test]
fn literal_length_builtin_on_string_and_array() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ 'hello' | length }}".into(), json!({}))
            .unwrap(),
        "5"
    );
    assert_eq!(
        env.render_string("{{ [1,2,3] | length }}".into(), json!({}))
            .unwrap(),
        "3"
    );
}

#[test]
fn is_test_empty_args_uses_variable_lhs() {
    let env = Environment::default();
    let ctx = json!({ "v": Value::Null });
    assert_eq!(
        env.render_string("{{ v is none }}".into(), ctx.clone())
            .unwrap(),
        "true"
    );
    assert_eq!(
        env.render_string("{{ missing is defined }}".into(), ctx)
            .unwrap(),
        "false"
    );
}

#[test]
fn variable_builtin_upper_lower_length_matches_expectation() {
    let env = Environment::default();
    let ctx = json!({
        "s": "Hello",
        "arr": [1, 2, 3],
        "obj": { "a": 1, "b": 2 }
    });
    assert_eq!(
        env.render_string("{{ s | upper }}".into(), ctx.clone())
            .unwrap(),
        "HELLO"
    );
    assert_eq!(
        env.render_string("{{ s | lower }}".into(), ctx.clone())
            .unwrap(),
        "hello"
    );
    assert_eq!(
        env.render_string("{{ s | length }}".into(), ctx.clone())
            .unwrap(),
        "5"
    );
    assert_eq!(
        env.render_string("{{ arr | length }}".into(), ctx.clone())
            .unwrap(),
        "3"
    );
    assert_eq!(
        env.render_string("{{ obj | length }}".into(), ctx).unwrap(),
        "2"
    );
}

#[test]
fn literal_length_fast_path_skipped_when_custom_filter_registered() {
    let mut env = Environment::default();
    env.add_filter(
        "length",
        Arc::new(|input, _args| Ok(Value::String(format!("custom:{}", value_to_string(input))))),
    );
    let out = env
        .render_string("{{ 'hi' | length }}".into(), json!({}))
        .unwrap();
    assert_eq!(out, "custom:hi");
}

#[test]
fn unary_not_on_missing_variable_is_true() {
    let env = Environment::default();
    let out = env
        .render_string("{{ not missing }}".into(), json!({}))
        .unwrap();
    assert_eq!(out, "true");
}

#[test]
fn unary_plus_on_string_variable_passthrough() {
    let env = Environment::default();
    let out = env
        .render_string("{{ +x }}".into(), json!({ "x": "plain" }))
        .unwrap();
    assert_eq!(out, "plain");
}

#[test]
fn unary_neg_on_string_errors() {
    let env = Environment::default();
    let err = env
        .render_string("{{ -x }}".into(), json!({ "x": "nope" }))
        .unwrap_err();
    assert!(
        err.to_string().contains("unary '-' expects"),
        "unexpected error: {err}"
    );
}

#[test]
fn nested_for_loop_objects_independent_per_inner_iteration() {
    let env = Environment::default();
    let tpl = concat!(
        "{% for a in outer %}",
        "{% for b in a %}",
        "{{ loop.index }}/{{ loop.index0 }}/{{ loop.length }}|",
        "{% endfor %}",
        "{% endfor %}",
    );
    let out = env
        .render_string(tpl.into(), json!({ "outer": [["x", "y"], ["z"]] }))
        .unwrap();
    assert_eq!(out, "1/0/2|2/1/2|1/0/1|");
}

#[test]
fn plain_text_preserves_unicode_and_empty_adjacent_segments() {
    let env = Environment::default();
    let out = env.render_string("日本語🙂".into(), json!({})).unwrap();
    assert_eq!(out, "日本語🙂");
    assert_eq!(env.render_string("".into(), json!({})).unwrap(), "");
}

#[test]
fn mixed_text_output_if_switch_concatenation_order_is_stable() {
    let env = Environment::default();
    let tpl = concat!(
        "A",
        "{{ x }}",
        "B",
        "{% if flag %}C{{ y }}{% endif %}",
        "D",
        "{% switch kind %}",
        "{% case 'ok' %}E{{ z }}",
        "{% default %}F",
        "{% endswitch %}",
        "G",
    );
    let out = env
        .render_string(
            tpl.into(),
            json!({ "x": 1, "y": 2, "z": 3, "flag": true, "kind": "ok" }),
        )
        .unwrap();
    assert_eq!(out, "A1BC2DE3G");
}

#[test]
fn set_in_for_updates_existing_outer_binding() {
    // Nunjucks: `{% set %}` targets the innermost frame that already has the name; outer `x` is
    // updated from inside the loop, so the final `{{ x }}` sees the last assignment.
    let env = Environment::default();
    let tpl = concat!(
        "{% set x = 1 %}",
        "{{ x }}",
        "{% for i in [1] %}",
        "{% set x = 2 %}",
        "{{ x }}",
        "{% endfor %}",
        "{{ x }}",
    );
    let out = env.render_string(tpl.into(), json!({})).unwrap();
    assert_eq!(out, "122");
}

#[test]
fn binary_add_two_context_variables() {
    let env = Environment::default();
    let ctx = json!({ "a": 10, "b": 32 });
    assert_eq!(
        env.render_string("{{ a + b }}".into(), ctx.clone())
            .unwrap(),
        "42"
    );
}

#[test]
fn binary_add_variable_and_literal() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ x + 2 }}".into(), json!({ "x": 40 }))
            .unwrap(),
        "42"
    );
}

#[test]
fn binary_mul_variables_no_extra_resolve_clone_regression() {
    let env = Environment::default();
    assert_eq!(
        env.render_string("{{ x * y }}".into(), json!({ "x": 6, "y": 7 }))
            .unwrap(),
        "42"
    );
}

#[test]
fn fused_lower_title_matches_builtin_title_on_lowered_string() {
    let env = Environment::default();
    let ctx = json!({ "s": "hello WORLD foo" });
    let fused = env
        .render_string("{{ s | lower | title }}".into(), ctx.clone())
        .unwrap();
    let sequential = env
        .render_string("{{ (s | lower) | title }}".into(), ctx)
        .unwrap();
    assert_eq!(fused, sequential);
    assert_eq!(fused, "Hello World Foo");
}

#[test]
fn title_fusion_skipped_when_custom_title_registered() {
    let mut env = Environment::default();
    env.add_filter(
        "title",
        Arc::new(|input, _args| Ok(Value::String(format!("custom:{}", value_to_string(input))))),
    );
    let out = env
        .render_string("{{ 'ab' | title }}".into(), json!({}))
        .unwrap();
    assert_eq!(out, "custom:ab");
}

#[test]
fn trim_fast_path_on_variable_matches_explicit_filter() {
    let env = Environment::default();
    let ctx = json!({ "s": "  hi  " });
    let a = env
        .render_string("{{ s | trim }}".into(), ctx.clone())
        .unwrap();
    let b = env
        .render_string("{{ s | trim | upper }}".into(), ctx)
        .unwrap();
    assert_eq!(a, "hi");
    assert_eq!(b, "HI");
}

#[test]
fn joiner_call_path_matches_comma_separated_output() {
    let env = Environment::default();
    let out = env
        .render_string(
            "{% set j = joiner(',') %}{{ j() }}a{{ j() }}b{{ j() }}c".into(),
            json!({}),
        )
        .unwrap();
    assert_eq!(out, "a,b,c");
}

#[test]
fn for_loop_without_loop_binding_still_renders_items() {
    let env = Environment::default();
    let tpl = "{% for n in nums %}{{ n }}{% endfor %}";
    let out = env
        .render_string(tpl.into(), json!({ "nums": [1, 2, 3] }))
        .unwrap();
    assert_eq!(out, "123");
}

#[test]
fn for_loop_with_loop_binding_keeps_metadata_semantics() {
    let env = Environment::default();
    let tpl = "{% for n in nums %}{{ loop.index0 }}:{{ n }}|{% endfor %}";
    let out = env
        .render_string(tpl.into(), json!({ "nums": [10, 20] }))
        .unwrap();
    assert_eq!(out, "0:10|1:20|");
}

#[test]
fn switch_uses_first_matching_case_and_fallthrough_on_empty_body() {
    let env = Environment::default();
    let tpl = concat!(
        "{% switch kind %}",
        "{% case 'x' %}",
        "{% case 'y' %}Y",
        "{% default %}D",
        "{% endswitch %}",
    );
    let out = env
        .render_string(tpl.into(), json!({ "kind": "x" }))
        .unwrap();
    assert_eq!(out, "Y");
}

#[test]
fn inline_if_with_variable_predicate_matches_expected_branch() {
    let env = Environment::default();
    let t = "{{ 'A' if flag else 'B' }}";
    assert_eq!(
        env.render_string(t.into(), json!({ "flag": true }))
            .unwrap(),
        "A"
    );
    assert_eq!(
        env.render_string(t.into(), json!({ "flag": false }))
            .unwrap(),
        "B"
    );
}

#[test]
fn repeated_renders_keep_loop_visible_inside_include() {
    let env = Environment::default();
    let ctx = json!({ "nums": [4, 5, 6] });
    let tpl = "{% for n in nums %}{{ loop.index0 }}:{{ n }}|{% endfor %}";
    let first = env.render_string(tpl.into(), ctx.clone()).unwrap();
    let second = env.render_string(tpl.into(), ctx).unwrap();
    assert_eq!(first, "0:4|1:5|2:6|");
    assert_eq!(second, "0:4|1:5|2:6|");
}

#[test]
fn macro_default_expression_not_evaluated_when_arg_is_passed() {
    let env = Environment::default();
    let tpl = concat!(
        "{% macro m(x=missing.attr) %}{{ x }}{% endmacro %}",
        "{{ m('ok') }}",
    );
    let out = env.render_string(tpl.into(), json!({})).unwrap();
    assert_eq!(out, "ok");
}

#[test]
fn caller_default_expression_not_evaluated_when_arg_is_passed() {
    let env = Environment::default();
    let tpl = concat!(
        "{% macro wrap() %}{{ caller('ok') }}{% endmacro %}",
        "{% call(x=missing.attr) wrap() %}{{ x }}{% endcall %}",
    );
    let out = env.render_string(tpl.into(), json!({})).unwrap();
    assert_eq!(out, "ok");
}

#[test]
fn caller_default_expression_can_see_prior_param_binding() {
    let env = Environment::default();
    let tpl = concat!(
        "{% macro wrap() %}{{ caller('seen') }}{% endmacro %}",
        "{% call(x, y=x) wrap() %}{{ y }}{% endcall %}",
    );
    let out = env.render_string(tpl.into(), json!({})).unwrap();
    assert_eq!(out, "seen");
}

#[test]
fn caller_output_order_and_outer_scope_remain_stable() {
    let env = Environment::default();
    let tpl = concat!(
        "{% set x = 'outer' %}",
        "{% macro wrap() %}[{{ caller('A') }}|{{ caller('B') }}]{% endmacro %}",
        "{% call(item) wrap() %}{{ item }}{{ x }}{% endcall %}",
        "|{{ x }}",
    );
    let out = env.render_string(tpl.into(), json!({})).unwrap();
    assert_eq!(out, "[Aouter|Bouter]|outer");
}

#[test]
fn macro_set_does_not_leak_back_to_caller_scope() {
    let env = Environment::default();
    let tpl = concat!(
        "{% set x = 'outer' %}",
        "{% macro mutate() %}{% set x = 'inner' %}{{ x }}{% endmacro %}",
        "{{ mutate() }}|{{ x }}",
    );
    let out = env.render_string(tpl.into(), json!({})).unwrap();
    assert_eq!(out, "inner|outer");
}

#[test]
fn macro_param_rebind_does_not_mutate_caller_binding() {
    let env = Environment::default();
    let tpl = concat!(
        "{% macro mutate(row) %}{% set row = 'inner' %}{{ row }}{% endmacro %}",
        "{{ mutate(row) }}|{{ row.msg }}",
    );
    let out = env
        .render_string(tpl.into(), json!({ "row": { "msg": "outer" } }))
        .unwrap();
    assert_eq!(out, "inner|outer");
}

#[test]
fn fused_filter_chain_on_attr_chain_matches_expected_output() {
    let env = Environment::default();
    let ctx = json!({
        "row": {
            "payload": {
                "title": "  Hello World  "
            }
        }
    });
    let out = env
        .render_string("{{ row.payload.title | trim | upper }}".into(), ctx)
        .unwrap();
    assert_eq!(out, "HELLO WORLD");
}

#[test]
fn attr_chain_filter_fast_path_respects_custom_overrides() {
    let mut env = Environment::default();
    env.add_filter(
        "upper",
        Arc::new(|input, _args| Ok(Value::String(format!("custom:{}", value_to_string(input))))),
    );
    let out = env
        .render_string(
            "{{ row.payload.title | upper }}".into(),
            json!({ "row": { "payload": { "title": "hello" } } }),
        )
        .unwrap();
    assert_eq!(out, "custom:hello");
}

#[test]
fn if_condition_on_attr_chain_uses_expected_branch() {
    let env = Environment::default();
    let tpl = "{% if row.enabled %}yes{% else %}no{% endif %}";
    assert_eq!(
        env.render_string(tpl.into(), json!({ "row": { "enabled": true } }))
            .unwrap(),
        "yes"
    );
    assert_eq!(
        env.render_string(tpl.into(), json!({ "row": { "enabled": false } }))
            .unwrap(),
        "no"
    );
}

#[test]
fn switch_on_attr_chain_matches_literal_case() {
    let env = Environment::default();
    let tpl = concat!(
        "{% switch row.kind %}",
        "{% case 'warn' %}W",
        "{% case 'ok' %}O",
        "{% default %}D",
        "{% endswitch %}",
    );
    let out = env
        .render_string(tpl.into(), json!({ "row": { "kind": "ok" } }))
        .unwrap();
    assert_eq!(out, "O");
}

#[test]
fn filter_block_captures_rendered_body_before_filtering() {
    let env = Environment::default();
    let tpl = concat!(
        "{% filter upper %}",
        "a{{ x }}",
        "{% if flag %}b{{ y }}{% endif %}",
        "{% endfilter %}",
    );
    let out = env
        .render_string(tpl.into(), json!({ "x": "m", "y": "n", "flag": true }))
        .unwrap();
    assert_eq!(out, "AMBN");
}

#[test]
fn extension_block_receives_rendered_body_string() {
    let mut env = Environment::default();
    env.register_extension(
        "capture",
        vec![("capture".into(), Some("endcapture".into()))],
        Arc::new(|_ctx, args, body| Ok(format!("{}=>{}", args.trim(), body.unwrap_or_default()))),
    )
    .unwrap();
    let tpl = concat!(
        "{% capture tagged %}",
        "a{{ x }}",
        "{% if flag %}b{{ y }}{% endif %}",
        "{% endcapture %}",
    );
    let out = env
        .render_string(tpl.into(), json!({ "x": 1, "y": 2, "flag": true }))
        .unwrap();
    assert_eq!(out, "tagged=&gt;a1b2");
}

#[test]
fn inline_if_on_attr_chain_uses_expected_branch() {
    let env = Environment::default();
    let tpl = "{{ 'up' if row.enabled else 'down' }}";
    assert_eq!(
        env.render_string(tpl.into(), json!({ "row": { "enabled": true } }))
            .unwrap(),
        "up"
    );
    assert_eq!(
        env.render_string(tpl.into(), json!({ "row": { "enabled": false } }))
            .unwrap(),
        "down"
    );
}
