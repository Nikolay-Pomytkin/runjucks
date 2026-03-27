import assert from 'node:assert/strict'
import test from 'node:test'
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
