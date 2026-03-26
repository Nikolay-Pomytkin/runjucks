/**
 * Parity: `renderStringFromJson` (JSON string context) vs `renderString` (object context).
 */
import assert from 'node:assert/strict'
import test from 'node:test'

import { Environment, renderString, renderStringFromJson } from '../index.js'

test('renderStringFromJson matches renderString (Environment)', () => {
  const env = new Environment()
  const tpl = 'Hello {{ name }}, {{ n }} items'
  const ctx = { name: 'Ada', n: 3 }
  const a = env.renderString(tpl, ctx)
  const b = env.renderStringFromJson(tpl, JSON.stringify(ctx))
  assert.equal(a, b)
})

test('renderStringFromJson matches renderString (module default env)', () => {
  const tpl = '{{ x }}'
  const ctx = { x: 42 }
  const a = renderString(tpl, ctx)
  const b = renderStringFromJson(tpl, JSON.stringify(ctx))
  assert.equal(a, b)
})
