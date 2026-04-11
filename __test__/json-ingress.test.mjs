/**
 * Parity: `renderStringFromJson` / `renderStringFromJsonBuffer` vs `renderString` (object context).
 */
import { Buffer } from 'node:buffer'
import assert from 'node:assert/strict'
import test from 'node:test'

import {
  Environment,
  renderString,
  renderStringFromJson,
  renderStringFromJsonBuffer,
} from '../index.js'

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

test('renderStringFromJsonBuffer matches renderString (Environment)', () => {
  const env = new Environment()
  const tpl = 'Hello {{ name }}, {{ n }} items'
  const ctx = { name: 'Ada', n: 3 }
  const a = env.renderString(tpl, ctx)
  const json = JSON.stringify(ctx)
  const b = env.renderStringFromJsonBuffer(tpl, Buffer.from(json, 'utf8'))
  assert.equal(a, b)
})

test('renderStringFromJsonBuffer matches renderStringFromJson (module)', () => {
  const tpl = '{{ x }}'
  const ctx = { x: 42 }
  const json = JSON.stringify(ctx)
  const a = renderStringFromJson(tpl, json)
  const b = renderStringFromJsonBuffer(tpl, Buffer.from(json, 'utf8'))
  assert.equal(a, b)
})

test('renderStringFromJsonBuffer accepts Uint8Array (e.g. TextEncoder)', () => {
  const tpl = '{{ x }}'
  const ctx = { x: 42 }
  const json = JSON.stringify(ctx)
  const bytes = new TextEncoder().encode(json)
  assert.ok(bytes instanceof Uint8Array)
  const a = renderStringFromJson(tpl, json)
  const b = renderStringFromJsonBuffer(tpl, bytes)
  assert.equal(a, b)
})

test('Environment.renderStringFromJsonBuffer accepts Uint8Array', () => {
  const env = new Environment()
  const tpl = '{{ x }}'
  const ctx = { x: 7 }
  const json = JSON.stringify(ctx)
  const bytes = new TextEncoder().encode(json)
  const a = env.renderString(tpl, ctx)
  const b = env.renderStringFromJsonBuffer(tpl, bytes)
  assert.equal(a, b)
})
