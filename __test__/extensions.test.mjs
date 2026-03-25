import assert from 'node:assert/strict'
import test from 'node:test'
import { Environment } from '../index.js'

test('addExtension: simple tag and args; body is null', () => {
  const env = new Environment()
  env.setAutoescape(false)
  env.addExtension('myext', ['echo'], null, (ctx, args, body) => {
    assert.equal(body, null)
    assert.equal(typeof ctx, 'object')
    return `[${args.trim()}]`
  })
  assert.equal(env.renderString('{% echo  a  b  %}', {}), '[a  b]')
})

test('addExtension: block tag with blocks map', () => {
  const env = new Environment()
  env.setAutoescape(false)
  env.addExtension(
    'wrap',
    ['wrap'],
    { wrap: 'endwrap' },
    (ctx, args, body) => `<${args.trim()}>${body}</${args.trim()}>`,
  )
  assert.equal(
    env.renderString('{% wrap box %}{{ x }}{% endwrap %}', { x: 1 }),
    '<box>1</box>',
  )
})

test('addExtension: process receives context keys', () => {
  const env = new Environment()
  env.setAutoescape(false)
  env.addExtension('e', ['show'], null, (ctx) => String(ctx?.n ?? 'missing'))
  assert.equal(env.renderString('{% show %}', { n: 7 }), '7')
})

test('addExtension: output is HTML-escaped when autoescape is on', () => {
  const env = new Environment()
  env.setAutoescape(true)
  env.addExtension('e', ['rawhtml'], null, () => '<em>x</em>')
  assert.equal(env.renderString('{% rawhtml %}', {}), '&lt;em&gt;x&lt;/em&gt;')
})

test('addExtension: empty tags list throws', () => {
  const env = new Environment()
  assert.throws(
    () => env.addExtension('e', [], null, () => ''),
    /at least one tag name/,
  )
})

test('addExtension: reserved built-in tag name throws', () => {
  const env = new Environment()
  assert.throws(
    () => env.addExtension('e', ['if'], null, () => ''),
    /built-in tag/,
  )
})

test('addExtension: second extension cannot steal tag name', () => {
  const env = new Environment()
  env.addExtension('a', ['echo'], null, () => 'a')
  assert.throws(
    () => env.addExtension('b', ['echo'], null, () => 'b'),
    /already registered/,
  )
})

test('addExtension: orphan end tag fails at render', () => {
  const env = new Environment()
  env.setAutoescape(false)
  env.addExtension('w', ['wrap'], { wrap: 'endwrap' }, () => '')
  assert.throws(
    () => env.renderString('{% endwrap %}', {}),
    /without matching opening/,
  )
})

test('hasExtension and removeExtension', () => {
  const env = new Environment()
  assert.equal(env.hasExtension('e'), false)
  env.setAutoescape(false)
  env.addExtension('e', ['show'], null, () => 'ok')
  assert.equal(env.hasExtension('e'), true)
  assert.equal(env.removeExtension('e'), true)
  assert.equal(env.hasExtension('e'), false)
  assert.equal(env.removeExtension('e'), false)
})
