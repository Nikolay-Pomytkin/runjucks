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

test('renderString: plain text without delimiters is unchanged', () => {
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

test('addFilter: JS callback runs with (input, ...args)', () => {
  const env = new Environment()
  env.addFilter('double', (s) => String(s) + String(s))
  assert.equal(env.renderString('{{ "a" | double }}', {}), 'aa')
})

test('addFilter: overrides built-in upper', () => {
  const env = new Environment()
  env.addFilter('upper', () => 'custom')
  assert.equal(env.renderString('{{ "x" | upper }}', {}), 'custom')
})

test('addGlobal: exposes JSON value to templates', () => {
  const env = new Environment()
  env.addGlobal('greeting', 'Hello')
  assert.equal(env.renderString('{{ greeting }}', {}), 'Hello')
})

test('configure: sets autoescape and dev', () => {
  const env = new Environment()
  env.configure({ autoescape: false, dev: true })
  assert.equal(env.renderString('{{ "<b>" }}', {}), '<b>')
})

test('configure: throwOnUndefined rejects missing variables', () => {
  const env = new Environment()
  env.configure({ throwOnUndefined: true })
  assert.throws(() => env.renderString('{{ nope }}', {}), /undefined variable/)
})

test('addTest: custom is test', () => {
  const env = new Environment()
  env.addTest('multiple_of_three', (v, n) => Number(n) !== 0 && Number(v) % Number(n) === 0)
  assert.equal(env.renderString('{{ 9 is multiple_of_three(3) }}', {}), 'true')
  assert.equal(env.renderString('{{ 10 is multiple_of_three(3) }}', {}), 'false')
})
