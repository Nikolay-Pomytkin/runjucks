/**
 * Proof for PER_TEMPLATE_AUTOESCAPE.md: Nunjucks can disable escaping per extension;
 * Runjucks applies global autoescape to extension output only.
 */
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import test from 'node:test'
import { Environment } from '../index.js'

const require = createRequire(import.meta.url)
const nunjucks = require('nunjucks')

test('reference: nunjucks extension with autoescape false keeps raw HTML when env autoescape true', () => {
  const nodes = nunjucks.nodes
  function TestExtension() {
    this.tags = ['test']
    this.autoescape = false
    this.parse = function parse(parser) {
      const tok = parser.nextToken()
      const args = parser.parseSignature(null, true)
      parser.advanceAfterBlockEnd(tok.value)
      return new nodes.CallExtension(this, 'run', args, null)
    }
    this.run = () => '<b>Foo</b>'
  }
  const env = new nunjucks.Environment([], { autoescape: true })
  env.addExtension('TestExtension', new TestExtension())
  const html = env.renderString('{% test "x" %}', {})
  assert.equal(html, '<b>Foo</b>')
})

test('runjucks: extension output is escaped when global autoescape true (no per-extension opt-out)', () => {
  const env = new Environment()
  env.setAutoescape(true)
  env.addExtension('e', ['rawhtml'], null, () => '<b>Foo</b>')
  assert.equal(env.renderString('{% rawhtml %}', {}), '&lt;b&gt;Foo&lt;/b&gt;')
})
