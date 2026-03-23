/**
 * TDD: Nunjucks-style `{{ var }}` behavior (expected to fail until lexer/parser land).
 *
 * Run: `npm run build && npm run test:pending`
 */
import assert from 'node:assert/strict'
import test from 'node:test'
import { renderString, Environment } from '../index.js'

test('pending: simple variable interpolation', () => {
  assert.equal(renderString('{{ msg }}', { msg: 'hi' }), 'hi')
})

test('pending: interpolation with surrounding text', () => {
  assert.equal(renderString('Hello, {{ name }}!', { name: 'Ada' }), 'Hello, Ada!')
})

test('pending: default autoescape on interpolated HTML', () => {
  assert.equal(
    renderString('{{ x }}', { x: '<script>' }),
    '&lt;script&gt;',
  )
})

test('pending: whitespace inside {{  }}', () => {
  assert.equal(renderString('{{  y  }}', { y: 'ok' }), 'ok')
})

test('pending: Environment matches renderString for interpolation', () => {
  const env = new Environment()
  assert.equal(env.renderString('{{ n }}', { n: 42 }), '42')
})
