/**
 * Hand-written scenarios for throughput vs template size / structure.
 * Each case: { name, template, context?, env? } — same shape as conformance fixtures.
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
      name: 'synth_if_nested',
      template: `{% if a %}{% if b %}{{ x }}{% else %}no{% endif %}{% else %}outer{% endif %}`,
      context: { a: true, b: true, x: 'ok' },
    },
  ]
}
