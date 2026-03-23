import assert from 'node:assert/strict'
import test from 'node:test'
import { renderString, Environment } from '../index.js'

test('renderString: empty template', () => {
  assert.equal(renderString('', {}), '')
})

test('renderString: multiline and unicode', () => {
  const s = 'line1\n你好\n🦀'
  assert.equal(renderString(s, {}), s)
})

test('renderString: context is ignored for plain-text-only lexer', () => {
  assert.equal(
    renderString('no substitution yet', { name: 'Ada', n: 42 }),
    'no substitution yet',
  )
})

test('Environment: toggling autoescape is stable for plain text', () => {
  const env = new Environment()
  env.setAutoescape(true)
  assert.equal(env.renderString('ok', {}), 'ok')
  env.setAutoescape(false)
  assert.equal(env.renderString('ok', {}), 'ok')
})

test('Environment: setDev does not throw', () => {
  const env = new Environment()
  env.setDev(true)
  assert.equal(env.renderString('x', {}), 'x')
  env.setDev(false)
})

test('addFilter is a no-op stub', () => {
  const env = new Environment()
  env.addFilter('noop', () => 'should not run')
  assert.equal(env.renderString('plain', {}), 'plain')
})
