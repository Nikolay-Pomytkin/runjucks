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
