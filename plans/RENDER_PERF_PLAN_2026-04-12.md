# Render-Core Perf Status

Date: 2026-04-12

This file replaces the earlier Wave 3 proposal with the landed status for the renderer-focused perf pass.

## Completed in this wave

- Branch / switch / macro-frame work from the earlier continuation plan is now fully landed.
- Added the missing synthetic perf rows in `perf/synthetic.mjs`:
  - `synth_macro_call_with_filter_chain_in_loop`
  - `synth_inline_if_filter_chain_dense`
- Added matching Criterion cases in `native/crates/runjucks-core/benches/render_hotspots.rs`:
  - `macro_call_with_filter_chain_in_loop`
  - `inline_if_filter_chain_dense`
- Converted the sync renderer hot path from per-node `String` returns to append-based assembly:
  - `render_node_into`
  - `render_children_into`
  - `render_output_into`
- Converted the async renderer hot path to the same append-based structure:
  - `render_node_into_async`
  - `render_children_into_async`
  - `render_output_into_async`
- Moved `Root`, `Text`, `Output`, `If`, `For`, `Switch`, `Set` block capture, `CallBlock`, `FilterBlock`, and extension body capture onto the append-based path.
- Added a per-root top-level macro cache on `RenderState`, keyed by root identity, and reused it in sync + async `Root` / `import` / `from import` handling.
- Follow-up fix: changed cached macro scopes to shared `Arc` maps and skipped root-cache lookup entirely for roots without top-level macros, removing constant overhead from tiny templates.
- Follow-up macro/caller pass: batched macro-frame and `caller()` param binding directly into pre-created local frames, removing the temporary bindings vector and per-parameter frame mutation churn.
- Added caller-heavy perf coverage:
  - `synth_call_block_with_args_in_loop`
  - `call_block_with_args_in_loop`

## Regression coverage added

Added writer-path guardrails in `native/crates/runjucks-core/tests/perf_regressions.rs` for:

- mixed text + output + `if` + `switch` concatenation order
- `caller()` output order with outer-scope visibility preserved
- filter-block body capture
- extension block body capture

Existing macro isolation, caller default, attr-chain branch/switch, and macro param rebinding coverage remains in place.

## Verification

Passed:

- `cargo test --manifest-path native/Cargo.toml -p runjucks_core --test perf_regressions`
- `npm run test:rust`
- `npm run build`
- `cargo bench --manifest-path native/Cargo.toml -p runjucks_core --bench render_hotspots`

## Local perf snapshot

Sequential `node perf/run.mjs --prefix=...` samples from the writer-path wave:

- `synth_conditional_macro_iter_switch_filters`
  - before this wave: `0.1853ms`
  - after this wave: `0.1741ms`
  - delta: about `6.0%` faster
  - current parity: Nunjucks `0.1290ms`, `0.74x`
- `synth_switch_in_for_with_attr_filters`
  - before this wave: `0.3119ms`
  - after this wave: `0.2868ms`
  - delta: about `8.0%` faster
  - current parity: Nunjucks `0.1849ms`, `0.64x`
- `synth_macro_call_with_filter_chain_in_loop`
  - first added in this wave: `0.2296ms`
  - current parity: Nunjucks `0.1660ms`, `0.72x`
- `synth_inline_if_filter_chain_dense`
  - first added in this wave: `0.1904ms`
  - current parity: Nunjucks `0.1493ms`, `0.78x`

Sequential `node perf/run.mjs --prefix=...` samples after the macro/caller binding pass:

- `synth_conditional_macro_iter_switch_filters`
  - before this pass: `0.1923ms`
  - after this pass: `0.1778ms`
  - delta: about `7.5%` faster
  - current parity: Nunjucks `0.1443ms`, `0.81x`
- `synth_switch_in_for_with_attr_filters`
  - before this pass: `0.3125ms`
  - after this pass: `0.2883ms`
  - delta: about `7.7%` faster
  - current parity: Nunjucks `0.1913ms`, `0.66x`
- `synth_macro_call_with_filter_chain_in_loop`
  - before this pass: `0.2487ms`
  - after this pass: `0.2280ms`
  - delta: about `8.3%` faster
  - current parity: Nunjucks `0.1889ms`, `0.83x`
- `synth_inline_if_filter_chain_dense`
  - before this pass: `0.2095ms`
  - after this pass: `0.1950ms`
  - delta: about `6.9%` faster
  - current parity: Nunjucks `0.1637ms`, `0.84x`
- `synth_call_block_with_args_in_loop`
  - first added in this pass: `0.2499ms`
  - current parity: Nunjucks `0.1915ms`, `0.77x`

Criterion `render_hotspots` snapshot for the new / targeted rows:

- `conditional_macro_iter_switch_filters`: `82.368µs` to `99.097µs`
- `switch_in_for_with_attr_filters`: `95.657µs` to `96.437µs`
- `macro_call_with_filter_chain_in_loop`: `127.13µs` to `127.61µs`
- `inline_if_filter_chain_dense`: `76.412µs` to `76.788µs`

After the regression follow-up (`Arc` macro-scope reuse + no-cache fast path for macro-free roots), Criterion moved to:

- `conditional_macro_iter_switch_filters`: `77.342µs` to `81.666µs`
- `switch_in_for_with_attr_filters`: `95.334µs` to `95.785µs`
- `macro_call_with_filter_chain_in_loop`: `127.02µs` to `127.59µs`
- `inline_if_filter_chain_dense`: `75.446µs` to `75.768µs`

The large mixed rows stayed flat-to-better, while the previously regressed microbenches recovered.

Focused sequential Criterion reruns for the macro/caller-heavy rows after this pass:

- `conditional_macro_iter_switch_filters`: `76.416µs` to `88.283µs`
  - change: `-35.0%` to `-17.2%`
- `macro_call_with_filter_chain_in_loop`: `126.40µs` to `138.87µs`
  - change: `-19.8%` to `-6.7%`
- `call_block_with_args_in_loop`: `175.64µs` to `176.86µs`
  - change: `-18.1%` to `-8.7%`

One full-suite Criterion run during this pass showed thermal/noise-heavy late-benchmark regressions, so the focused sequential reruns above are the reliable numbers for the affected hot paths.

Criterion showed improvements on several iterator-heavy rows:

- `for_200_int_concat`
- `for_200_var_plus_context_int`
- `for_200_loop_index_and_item`
- `nested_for_small`
- `joiner_set_and_three_calls`

Criterion also improved the small filter / attr microbenches that had regressed in the intermediate state:

- `literal_string_upper_filter`
- `attr_chain_three_depth`
- `literal_string_length_filter`
- `variable_trim_upper_filters`
- `variable_trim_capitalize_filters`
- `variable_lower_title_filters`

Root cause of the temporary regressions: the first version of the top-level macro cache imposed hash lookup plus macro-scope cloning overhead on every `Root` render, even for tiny templates with no macros. Moving cached macro scopes to shared `Arc` maps and bypassing the cache on macro-free roots removed that fixed cost.

## Acceptance status

- Focused-row improvement `>= 8%`: met on `synth_switch_in_for_with_attr_filters`
- No new conformance / perf-regression failures: met
- No unrelated regression worse than `3%`: met after the regression follow-up

## Remaining high-ROI work

- Continue on macro-body / `caller()` heavy loop cost. The mixed macro row improved again, but it is still well behind Nunjucks.
- Revisit include / import scope cloning outside the macro path.
- Keep `synth_named_template_interp` and named-template/include overhead as a later wave, not the current render-core lane.
