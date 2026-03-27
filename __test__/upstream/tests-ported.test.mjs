/**
 * Ported from vendored `nunjucks/tests/tests.js` (Mocha → `node:test`).
 * Skipped cases: unknown `is` tests in Runjucks, `sameas` object identity, Map/Set/generator.
 * Callable case uses `addGlobal` because NAPI context is JSON-shaped (functions use P1 globals).
 */
import test from 'node:test'

import {
  assert,
  assertRendered,
  createUpstreamEnvironment,
  renderUpstream,
} from './upstream-harness.mjs'

test.describe('upstream nunjucks/tests/tests.js (ported)', () => {
  test('callable should detect callability (via addGlobal; context is JSON-only in NAPI)', () => {
    const env = createUpstreamEnvironment()
    env.addGlobal('foo', () => '!!!')
    env.addGlobal('bar', '!!!')
    assertRendered(assert, renderUpstream(env, '{{ foo is callable }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ bar is not callable }}', {}), 'true')
  })

  test('filter names are not callable or defined values in Nunjucks 3.x', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ upper is callable }}', {}), 'false')
    assertRendered(assert, renderUpstream(env, '{{ upper is defined }}', {}), 'false')
  })

  test('defined should detect definedness', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ foo is defined }}', {}), 'false')
    assertRendered(assert, renderUpstream(env, '{{ foo is not defined }}', {}), 'true')
    assertRendered(
      assert,
      renderUpstream(env, '{{ foo is defined }}', { foo: null }),
      'true',
    )
    assertRendered(
      assert,
      renderUpstream(env, '{{ foo is not defined }}', { foo: null }),
      'false',
    )
  })

  test('should support "is defined" in {% if %} expressions', () => {
    const env = createUpstreamEnvironment()
    assertRendered(
      assert,
      renderUpstream(
        env,
        '{% if foo is defined %}defined{% else %}undefined{% endif %}',
        {},
      ),
      'undefined',
    )
    assertRendered(
      assert,
      renderUpstream(
        env,
        '{% if foo is defined %}defined{% else %}undefined{% endif %}',
        { foo: null },
      ),
      'defined',
    )
  })

  test('should support "is not defined" in {% if %} expressions', () => {
    const env = createUpstreamEnvironment()
    assertRendered(
      assert,
      renderUpstream(
        env,
        '{% if foo is not defined %}undefined{% else %}defined{% endif %}',
        {},
      ),
      'undefined',
    )
    assertRendered(
      assert,
      renderUpstream(
        env,
        '{% if foo is not defined %}undefined{% else %}defined{% endif %}',
        { foo: null },
      ),
      'defined',
    )
  })

  test.skip(
    'undefined should detect undefinedness (needs `undefined` is test — NUNJUCKS_PARITY)',
    () => {},
  )

  test('none/null should detect strictly null values', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ null is null }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ none is none }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ none is null }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ foo is null }}', {}), 'false')
    assertRendered(
      assert,
      renderUpstream(env, '{{ foo is not null }}', { foo: null }),
      'false',
    )
  })

  test('divisibleby should detect divisibility', () => {
    const env = createUpstreamEnvironment()
    // Nunjucks coerces string operands; Runjucks `is` tests require numeric LHS (see NUNJUCKS_PARITY).
    assertRendered(assert, renderUpstream(env, '{{ 6 is divisibleby(3) }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ 3 is not divisibleby(2) }}', {}), 'true')
  })

  test.skip(
    'escaped should test whether or not something is escaped (needs `escaped` is test)',
    () => {},
  )

  test('even should detect whether or not a number is even', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ 5 is even }}', {}), 'false')
    assertRendered(assert, renderUpstream(env, '{{ 4 is not even }}', {}), 'false')
  })

  test('odd should detect whether or not a number is odd', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ 5 is odd }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ 4 is not odd }}', {}), 'true')
  })

  test.skip(
    'mapping should detect Maps or hashes (needs `mapping` is test + Map in context)',
    () => {},
  )

  test('falsy should detect whether or not a value is falsy', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ 0 is falsy }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ "pancakes" is not falsy }}', {}), 'true')
  })

  test('truthy should detect whether or not a value is truthy', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ null is truthy }}', {}), 'false')
    assertRendered(assert, renderUpstream(env, '{{ "pancakes" is not truthy }}', {}), 'false')
  })

  test.skip(
    'greaterthan / ge / lessthan / le / ne (needs Jinja-style comparison is tests)',
    () => {},
  )

  test.skip(
    'iterable (needs `iterable` is test — parity backlog)',
    () => {},
  )

  test('number should detect whether a value is numeric', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ 5 is number }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ "42" is number }}', {}), 'false')
  })

  test('string should detect whether a value is a string', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ 5 is string }}', {}), 'false')
    assertRendered(assert, renderUpstream(env, '{{ "42" is string }}', {}), 'true')
  })

  test('equalto should detect value equality', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ 1 is equalto(2) }}', {}), 'false')
    assertRendered(assert, renderUpstream(env, '{{ 2 is not equalto(2) }}', {}), 'false')
  })

  test.skip(
    'sameas should alias to equalto for same object reference (Runjucks: distinct objects compare false — NUNJUCKS_PARITY)',
    () => {},
  )

  test('lower should detect whether or not a string is lowercased', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ "foobar" is lower }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ "Foobar" is lower }}', {}), 'false')
  })

  test('upper should detect whether or not a string is uppercased', () => {
    const env = createUpstreamEnvironment()
    assertRendered(assert, renderUpstream(env, '{{ "FOOBAR" is upper }}', {}), 'true')
    assertRendered(assert, renderUpstream(env, '{{ "Foobar" is upper }}', {}), 'false')
  })
})
