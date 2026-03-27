/**
 * Cherry-picked from vendored `nunjucks/tests/filters.js` (sync `equal()` only).
 * Skips: async filters, `r.markSafe` / complex runtime objects, `lib.repeat`-sized `center`,
 * cases requiring `toString` on plain objects for escape.
 */
import test from 'node:test'

import {
  assert,
  assertRendered,
  createUpstreamEnvironment,
  renderUpstream,
} from './upstream-harness.mjs'

test.describe('upstream nunjucks/tests/filters.js (cherry-picked)', () => {
  test('abs', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ -3|abs }}', {}), '3')
    assertRendered(assert, renderUpstream(env, '{{ -3.456|abs }}', {}), '3.456')
  })

  test('batch', () => {
    const env = createUpstreamEnvironment()
    const tpl = [
      '{% for a in [1,2,3,4,5,6]|batch(2) %}',
      '-{% for b in a %}',
      '{{ b }}',
      '{% endfor %}-',
      '{% endfor %}',
    ].join('')
    assertRendered(assert, renderUpstream(env, tpl, {}), '-12--34--56-')
  })

  test('capitalize (string / null / missing)', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ "foo" | capitalize }}', {}), 'Foo')
    assertRendered(assert, renderUpstream(env, '{{ undefined | capitalize }}', {}), '')
    assertRendered(assert, renderUpstream(env, '{{ null | capitalize }}', {}), '')
    assertRendered(assert, renderUpstream(env, '{{ nothing | capitalize }}', {}), '')
  })

  test('default', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ undefined | default("foo") }}', {}), 'foo')
    assertRendered(
      assert,
      renderUpstream(env, '{{ bar | default("foo") }}', { bar: null }),
      '',
    )
    assertRendered(assert, renderUpstream(env, '{{ false | default("foo") }}', {}), 'false')
    assertRendered(assert, renderUpstream(env, '{{ false | default("foo", true) }}', {}), 'foo')
    assertRendered(assert, renderUpstream(env, '{{ bar | default("foo") }}', {}), 'foo')
    assertRendered(assert, renderUpstream(env, '{{ "bar" | default("foo") }}', {}), 'bar')
  })

  test('dictsort (by key, default)', () => {
    const env = createUpstreamEnvironment()
    const tpl =
      '{% for item in items | dictsort %}' + '{{ item[0] }}' + '{% endfor %}'
    const ctx = {
      items: { e: 1, d: 2, c: 3, a: 4, f: 5, b: 6 },
    }
    assertRendered(assert, renderUpstream(env, tpl, ctx), 'abcdef')
  })

  test('escape (autoescape off)', () => {
    const env = createUpstreamEnvironment({ autoescape: false })
    assertRendered(
      assert,
      renderUpstream(env, '{{ "<html>\\\\" | escape }}', {}),
      '&lt;html&gt;&#92;',
    )
  })

  test('escape skip safe / double escape', () => {
    const env = createUpstreamEnvironment({ autoescape: false })
    assertRendered(
      assert,
      renderUpstream(env, '{{ "<html>" | safe | escape }}', {}),
      '<html>',
    )
    assertRendered(
      assert,
      renderUpstream(env, '{{ "<html>" | escape | escape }}', {}),
      '&lt;html&gt;',
    )
  })

  test('escape with autoescape on (set + output)', () => {
    const env = createUpstreamEnvironment({ autoescape: true })
    assertRendered(
      assert,
      renderUpstream(env, '{% set val = "<html>" | escape %}{{ val }}', {}),
      '&lt;html&gt;',
    )
  })

  test('first', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ [1,2,3] | first }}', {}), '1')
  })

  test('float', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ "3.5" | float }}', {}), '3.5')
    assertRendered(assert, renderUpstream(env, '{{ "0" | float }}', {}), '0')
  })

  test('float (default value)', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ "bob" | float("cat") }}', {}), 'cat')
  })

  test('forceescape', () => {
    const env = createUpstreamEnvironment()
    assertRendered(
      assert,
      renderUpstream(env, '{{ "<html>" | safe | forceescape }}', {}),
      '&lt;html&gt;',
    )
  })

  test('int', () => {
    const env = createUpstreamEnvironment()
    // `int` uses integer parse for base 10 — `"3.5"` is not truncated like Nunjucks (parity partial).
    assertRendered(assert, renderUpstream(env, '{{ "3" | int }}', {}), '3')
    assertRendered(assert, renderUpstream(env, '{{ "0" | int }}', {}), '0')
    assertRendered(assert, renderUpstream(env, '{{ "foobar" | int("42") }}', {}), '42')
    // Keyword `base=` is not parsed; positional default + radix matches core (`filter_int`).
    assertRendered(assert, renderUpstream(env, '{{ "4d32" | int(0, 16) }}', {}), '19762')
    assertRendered(assert, renderUpstream(env, '{{ "011" | int(0, 8) }}', {}), '9')
  })

  test('int (default value)', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ "bob" | int("cat") }}', {}), 'cat')
  })

  test('indent (plain strings)', () => {
    const env = createUpstreamEnvironment()
    assertRendered(
      assert,
      renderUpstream(env, '{{ "one\ntwo\nthree" | indent }}', {}),
      'one\n    two\n    three',
    )
    assertRendered(
      assert,
      renderUpstream(env, '{{ "one\ntwo\nthree" | indent(2) }}', {}),
      'one\n  two\n  three',
    )
    assertRendered(
      assert,
      renderUpstream(env, '{{ "one\ntwo\nthree" | indent(2, true) }}', {}),
      '  one\n  two\n  three',
    )
    assertRendered(assert, renderUpstream(env, '{{ "" | indent }}', {}), '')
    assertRendered(assert, renderUpstream(env, '{{ undefined | indent }}', {}), '')
    assertRendered(assert, renderUpstream(env, '{{ null | indent }}', {}), '')
    assertRendered(assert, renderUpstream(env, '{{ nothing | indent }}', {}), '')
  })

  test('join', () => {
    const env = createUpstreamEnvironment()
    assertRendered(
      assert,
      renderUpstream(env, '{{ items | join }}', { items: [1, 2, 3] }),
      '123',
    )
    assertRendered(
      assert,
      renderUpstream(env, '{{ items | join(",") }}', {
        items: ['foo', 'bar', 'bear'],
      }),
      'foo,bar,bear',
    )
  })
})
