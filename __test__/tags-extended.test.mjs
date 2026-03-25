import assert from 'node:assert/strict'
import test from 'node:test'
import { Environment, renderString } from '../index.js'

test('renderString: switch and multi-set', () => {
  assert.equal(
    renderString(
      '{% switch 2 %}{% case 1 %}A{% case 2 %}B{% endswitch %}{% set x, y = "z" %}{{ x }}{{ y }}',
      {},
    ),
    'Bzz',
  )
})

test('Environment: include ignore missing + dynamic name via map', () => {
  const env = new Environment()
  env.setTemplateMap({
    'main.html': '{% include name ignore missing %}|{% include "part.html" %}',
    'part.html': 'ok',
  })
  assert.equal(env.renderTemplate('main.html', { name: 'missing.njk' }), '|ok')
})

test('Environment: loop visible inside included template', () => {
  const env = new Environment()
  env.setTemplateMap({
    'main.njk': '{% for item in [1,2] %}{% include "row.njk" %}{% endfor %}',
    'row.njk': '{{ loop.index }},{{ loop.first }}\n',
  })
  assert.equal(env.renderTemplate('main.njk', {}), '1,true\n2,false\n')
})

test('Environment: extends + block override', () => {
  const env = new Environment()
  env.setTemplateMap({
    'base.html':
      '<!doctype><title>{% block title %}T{% endblock %}</title><body>{% block body %}{% endblock %}</body>',
    'child.html':
      '{% extends "base.html" %}{% block title %}Hi{% endblock %}{% block body %}B{% endblock %}',
  })
  assert.equal(
    env.renderTemplate('child.html', {}),
    '<!doctype><title>Hi</title><body>B</body>',
  )
})

test('Environment: multi-level extends with super()', () => {
  const env = new Environment()
  env.setTemplateMap({
    'g.html': '{% block b %}G{% endblock %}',
    'p.html':
      '{% extends "g.html" %}{% block b %}P{{ super() }}{% endblock %}',
    'c.html':
      '{% extends parent %}{% block b %}C{{ super() }}{% endblock %}',
  })
  assert.equal(env.renderTemplate('c.html', { parent: 'p.html' }), 'CPG')
})

test('renderString: unclosed if tag throws', () => {
  assert.throws(
    () => {
      const env = new Environment()
      env.renderString('{% if x %}', {})
    },
    (err) =>
      err instanceof Error &&
      err.message.includes('endif') &&
      err.message.includes('if'),
  )
})

test.skip(
  'addFilter real callback (pending NAPI wiring)',
  () => {
    const env = new Environment()
    env.addFilter('double', (s) => `${s}${s}`)
    assert.equal(env.renderString('{{ "a" | double }}', {}), 'aa')
  },
)
