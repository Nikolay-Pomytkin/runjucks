import assert from 'node:assert/strict'
import test from 'node:test'
import { Environment } from '../index.js'

test('setLoaderCallback resolves template source', () => {
  const env = new Environment()
  const map = { 'x.njk': 'Hello {{ n }}' }
  env.setLoaderCallback((name) => map[name] ?? null)
  assert.equal(env.renderTemplate('x.njk', { n: 1 }), 'Hello 1')
})

test('setLoaderCallback null means missing template', () => {
  const env = new Environment()
  env.setLoaderCallback(() => null)
  assert.throws(() => env.renderTemplate('nope.njk', {}), /template not found/)
})

test('invalidateCache is callable', () => {
  const env = new Environment()
  env.setTemplateMap({ 'a.njk': '1' })
  env.renderTemplate('a.njk', {})
  env.invalidateCache()
  assert.equal(env.renderTemplate('a.njk', {}), '1')
})
