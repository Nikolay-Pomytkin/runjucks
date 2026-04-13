/**
 * Hand-written scenarios for throughput vs template size / structure.
 * Each case: { name, template?, context?, env?, renderMode?, templateName? } — same shape as
 * conformance fixtures when using inline `template`. Use `renderMode: 'template'` +
 * `templateName` + `env.templateMap` to benchmark `renderTemplate` / Nunjucks `env.render`.
 */

const nums = Array.from({ length: 200 }, (_, i) => i)

export function syntheticCases() {
  return [
    {
      name: 'synth_plain_small',
      template: 'hello world',
      context: {},
    },
    {
      name: 'synth_plain_large',
      template: 'x'.repeat(8000),
      context: {},
    },
    {
      name: 'synth_many_vars',
      template: Array.from({ length: 80 }, (_, i) => `{{ v${i} }}`).join(''),
      context: Object.fromEntries(
        Array.from({ length: 80 }, (_, i) => [`v${i}`, String(i)]),
      ),
    },
    {
      name: 'synth_for_medium',
      template: '{% for n in nums %}{{ n }}{% endfor %}',
      context: { nums },
    },
    {
      name: 'synth_filters_chain',
      template:
        '{{ "hello world" | upper | replace("WORLD", "runjucks") | length }}',
      context: {},
    },
    {
      name: 'synth_var_trim_upper',
      template: '{{ msg | trim | upper }}',
      context: { msg: '  hello  ' },
    },
    {
      name: 'synth_var_trim_capitalize',
      template: '{{ msg | trim | capitalize }}',
      context: { msg: '  hELLO  ' },
    },
    {
      name: 'synth_if_nested',
      template: `{% if a %}{% if b %}{{ x }}{% else %}no{% endif %}{% else %}outer{% endif %}`,
      context: { a: true, b: true, x: 'ok' },
    },
    {
      name: 'synth_nested_for',
      template:
        '{% for a in outer %}{% for b in a %}{{ b }}{% endfor %}{% endfor %}',
      context: { outer: [[1, 2], [3, 4]] },
    },
    {
      name: 'synth_long_var_lines',
      template: Array.from({ length: 50 }, () => '{{ x }}\n').join(''),
      context: { x: 'ok' },
    },
    {
      name: 'synth_deep_if_chain',
      template:
        '{% if a %}{% if b %}{% if c %}x{% endif %}{% endif %}{% endif %}',
      context: { a: true, b: true, c: true },
    },
    {
      name: 'synth_conditional_macro_iter_switch_filters',
      template:
        "{% macro fmt(v) -%}{% switch v.kind %}{% case 'warn' %}{{ v.msg | trim | upper }}{% case 'ok' %}{{ v.msg | trim | lower }}{% default %}{{ v.msg | trim }}{% endswitch %}{%- endmacro %}{% for v in rows %}{% if v.enabled %}{{ fmt(v) }}{% endif %}{% endfor %}",
      context: {
        rows: Array.from({ length: 80 }, (_, i) => ({
          enabled: i % 3 !== 0,
          kind: i % 2 === 0 ? 'warn' : 'ok',
          msg: `  Row ${i}  `,
        })),
      },
    },
    {
      name: 'synth_switch_in_for_with_attr_filters',
      template:
        "{% for row in rows %}{% switch row.type %}{% case 'a' %}{{ row.payload.title | trim | upper }}{% case 'b' %}{{ row.payload.title | trim | lower }}{% default %}{{ row.payload.title | trim }}{% endswitch %}{% endfor %}",
      context: {
        rows: Array.from({ length: 120 }, (_, i) => ({
          type: i % 3 === 0 ? 'a' : i % 3 === 1 ? 'b' : 'c',
          payload: { title: `  T${i}  ` },
        })),
      },
    },
    {
      name: 'synth_macro_call_with_filter_chain_in_loop',
      template:
        "{% macro fmt(v) -%}{{ v.msg | trim | upper }}|{{ v.alt | trim | lower }}{%- endmacro %}{% for v in rows %}{{ fmt(v) }}{% endfor %}",
      context: {
        rows: Array.from({ length: 100 }, (_, i) => ({
          msg: `  Row ${i}  `,
          alt: `  ALT ${i}  `,
        })),
      },
    },
    {
      name: 'synth_inline_if_filter_chain_dense',
      template:
        "{% for row in rows %}{{ (row.msg | trim | upper) if row.enabled else (row.msg | trim | lower) }}{% endfor %}",
      context: {
        rows: Array.from({ length: 120 }, (_, i) => ({
          enabled: i % 2 === 0,
          msg: `  Dense ${i}  `,
        })),
      },
    },
    {
      name: 'synth_call_block_with_args_in_loop',
      template:
        "{% macro wrap(items) -%}{% for item in items %}[{{ caller(item) }}]{% endfor %}{%- endmacro %}{% call(row, suffix='!') wrap(rows) %}{{ row.msg | trim | upper }}{{ suffix }}{% endcall %}",
      context: {
        rows: Array.from({ length: 100 }, (_, i) => ({
          msg: `  Call ${i}  `,
        })),
      },
    },
    {
      name: 'synth_named_template_interp',
      renderMode: 'template',
      templateName: 'main.njk',
      env: {
        templateMap: {
          'main.njk': '{{ a }},{{ b }}',
        },
      },
      context: { a: 'x', b: 'y' },
    },
    {
      name: 'synth_named_template_static',
      renderMode: 'template',
      templateName: 'main.njk',
      env: {
        templateMap: {
          'main.njk': 'ok',
        },
      },
      context: {},
    },
  ]
}

/**
 * Scenarios for `perf/run-async.mjs` — **Runjucks only** (`renderStringAsync` / `renderTemplateAsync`).
 * No Nunjucks baseline (upstream async API differs).
 */
export function asyncSyntheticCases() {
  return [
    {
      name: 'async_synth_plain',
      template: 'hello {{ x }}',
      context: { x: 'world' },
    },
    {
      name: 'async_synth_asyncEach_200',
      template: '{% asyncEach n in nums %}{{ n }}{% endeach %}',
      context: { nums },
    },
    {
      name: 'async_synth_asyncAll_200',
      template: '{% asyncAll n in nums %}{{ n }}{% endall %}',
      context: { nums },
    },
    {
      name: 'async_synth_named_template',
      renderMode: 'template',
      templateName: 'main.njk',
      env: {
        templateMap: {
          'main.njk': '{% for i in items %}{{ i }}{% endfor %}',
        },
      },
      context: { items: nums.slice(0, 50) },
    },
  ]
}

/**
 * Same template measured with sync `renderString` vs `renderStringAsync` (overhead hint).
 */
export function asyncSyncParityCases() {
  const tpl = '{% for n in nums %}{{ n }}{% endfor %}'
  const ctx = { nums }
  return [{ name: 'for_200_sync_vs_async', template: tpl, context: ctx }]
}
