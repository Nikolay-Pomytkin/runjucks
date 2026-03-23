import assert from 'node:assert/strict'
import test from 'node:test'
import { renderString, Environment } from '../index.js'

test('renderString returns plain text unchanged', () => {
  const out = renderString('hello world', {})
  assert.equal(out, 'hello world')
})

test('Environment.renderString matches top-level renderString', () => {
  const env = new Environment()
  assert.equal(env.renderString('x', {}), renderString('x', {}))
})

test('Environment setAutoescape does not break plain text', () => {
  const env = new Environment()
  env.setAutoescape(false)
  assert.equal(env.renderString('plain', {}), 'plain')
})
