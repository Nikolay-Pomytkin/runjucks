import assert from 'node:assert/strict'
import test from 'node:test'
import { Environment } from '../index.js'
import { serializeContextForRender } from '../serialize-context.js'

test('serializeContextForRender converts Map and Set', () => {
  const ctx = serializeContextForRender({
    m: new Map([
      ['a', 1],
      ['b', 2],
    ]),
    s: new Set([3, 4]),
  })
  assert.deepEqual(ctx, { m: { a: 1, b: 2 }, s: [3, 4] })
})

test('serializeContextForRender handles nested Map/Set and empty collections', () => {
  const ctx = serializeContextForRender({
    outer: new Map([
      [
        'inner',
        new Map([
          ['k', new Set([1, 1, 2])],
        ]),
      ],
    ]),
    emptyM: new Map(),
    emptyS: new Set(),
  })
  assert.deepEqual(ctx, {
    outer: { inner: { k: [1, 2] } },
    emptyM: {},
    emptyS: [],
  })
})

test('serializeContextForRender + render yields stable length and lookups', () => {
  const env = new Environment()
  env.setTemplateMap({
    t: '{{ m.a }} {{ s | length }} {{ empty | length }}',
  })
  const raw = {
    m: new Map([['a', 7]]),
    s: new Set(['x', 'y']),
    empty: new Set(),
  }
  const ctx = serializeContextForRender(raw)
  const html = env.renderTemplate('t', ctx)
  assert.equal(html, '7 2 0')
})
