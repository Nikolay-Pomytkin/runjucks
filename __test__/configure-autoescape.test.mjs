import assert from 'node:assert/strict'
import test from 'node:test'
import { Environment } from '../index.js'

test('configure autoescape non-empty string is truthy (Nunjucks-like)', () => {
  const env = new Environment()
  env.configure({ autoescape: 'html' })
  assert.equal(env.renderString('{{ x }}', { x: '<' }), '&lt;')
})

test('configure autoescape empty string is falsy', () => {
  const env = new Environment()
  env.configure({ autoescape: '' })
  assert.equal(env.renderString('{{ x }}', { x: '<' }), '<')
})

test('configure autoescape zero is falsy', () => {
  const env = new Environment()
  env.configure({ autoescape: 0 })
  assert.equal(env.renderString('{{ x }}', { x: '<' }), '<')
})

test('configure autoescape null is falsy', () => {
  const env = new Environment()
  env.configure({ autoescape: null })
  assert.equal(env.renderString('{{ x }}', { x: '<' }), '<')
})

test('configure autoescape false disables escaping', () => {
  const env = new Environment()
  env.configure({ autoescape: false })
  assert.equal(env.renderString('{{ x }}', { x: '<' }), '<')
})
