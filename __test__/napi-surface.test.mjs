/**
 * Smoke: public Environment API matches the curated list (keep in sync with index.d.ts).
 */
import assert from 'node:assert/strict'
import test from 'node:test'

import { Environment } from '../index.js'

const ENV_METHODS = [
  'renderString',
  'renderStringFromJson',
  'setAutoescape',
  'setDev',
  'setRandomSeed',
  'addFilter',
  'addTest',
  'addExtension',
  'hasExtension',
  'getExtension',
  'removeExtension',
  'addGlobal',
  'configure',
  'setLoaderCallback',
  'invalidateCache',
  'getTemplate',
  'setTemplateMap',
  'setLoaderRoot',
  'renderTemplate',
]

test('Environment exposes expected NAPI methods', () => {
  const env = new Environment()
  for (const name of ENV_METHODS) {
    assert.equal(
      typeof env[name],
      'function',
      `expected env.${name} to be a function`,
    )
  }
})
