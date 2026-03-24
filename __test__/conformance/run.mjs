/**
 * Runs JSON conformance fixtures against the published JS API (`renderString`).
 * Same files as `native/fixtures/conformance/*.json` (relative to repo root).
 */
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

import { Environment, renderString } from '../../index.js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '../../native/fixtures/conformance')

function loadCases() {
  const a = JSON.parse(readFileSync(join(root, 'render_cases.json'), 'utf8'))
  const b = JSON.parse(readFileSync(join(root, 'filter_cases.json'), 'utf8'))
  return [...a, ...b]
}

for (const c of loadCases()) {
  test(`conformance: ${c.id}`, () => {
    let out
    if (c.env) {
      const env = new Environment()
      if (c.env.autoescape === false) env.setAutoescape(false)
      if (c.env.dev === true) env.setDev(true)
      out = env.renderString(c.template, c.context ?? {})
    } else {
      out = renderString(c.template, c.context ?? {})
    }
    assert.equal(out, c.expected)
  })
}
