/**
 * Smoke: package.json `exports` resolve for main and documented subpaths.
 */
import assert from 'node:assert/strict'
import { createRequire } from 'node:module'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const require = createRequire(join(root, 'index.js'))

test('package.json exports lists required subpaths', () => {
  const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8'))
  assert.ok(pkg.exports['.'])
  assert.ok(pkg.exports['./express'])
  assert.ok(pkg.exports['./serialize-context'])
  assert.ok(pkg.exports['./fetch-template-map'])
  assert.ok(pkg.exports['./install-jinja-compat'])
  assert.ok(pkg.exports['./render-with-callback'])
})

test('require resolves @zneep/runjucks subpaths to files', () => {
  assert.equal(
    require.resolve('@zneep/runjucks/express'),
    join(root, 'express.js'),
  )
  assert.equal(
    require.resolve('@zneep/runjucks/serialize-context'),
    join(root, 'serialize-context.js'),
  )
  assert.equal(
    require.resolve('@zneep/runjucks/fetch-template-map'),
    join(root, 'fetch-template-map.js'),
  )
  assert.equal(
    require.resolve('@zneep/runjucks/install-jinja-compat'),
    join(root, 'install-jinja-compat.js'),
  )
  assert.equal(
    require.resolve('@zneep/runjucks/render-with-callback'),
    join(root, 'render-with-callback.js'),
  )
})

test('exported modules load', () => {
  assert.equal(typeof require('@zneep/runjucks/express').expressEngine, 'function')
  assert.equal(
    typeof require('@zneep/runjucks/serialize-context').serializeContextForRender,
    'function',
  )
  assert.equal(
    typeof require('@zneep/runjucks/fetch-template-map').fetchTemplateMap,
    'function',
  )
  assert.equal(
    typeof require('@zneep/runjucks/install-jinja-compat').installJinjaCompat,
    'function',
  )
  const rwc = require('@zneep/runjucks/render-with-callback')
  assert.equal(typeof rwc.renderWithCallback, 'function')
  assert.equal(typeof rwc.renderWithCallbackAsync, 'function')
})
