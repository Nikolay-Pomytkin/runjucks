/**
 * Runs JSON conformance fixtures against the published JS API (`renderString` / `Environment`).
 * Same files as `native/fixtures/conformance/*.json` (relative to repo root).
 */
import assert from 'node:assert/strict'
import test from 'node:test'

import { Environment, renderString } from '../../index.js'
import { loadAllConformanceCases } from './load-fixtures.mjs'

for (const c of loadAllConformanceCases()) {
  const label = `conformance [${c._suite}] ${c.id}`
  test(
    label,
    { skip: c.skip === true },
    () => {
      let out
      if (c.env) {
        const env = new Environment()
        if (c.env.autoescape === false) env.setAutoescape(false)
        if (c.env.dev === true) env.setDev(true)
        if (c.env.globals) {
          for (const [name, value] of Object.entries(c.env.globals)) {
            env.addGlobal(name, value)
          }
        }
        if (c.env.randomSeed != null && typeof env.setRandomSeed === 'function') {
          env.setRandomSeed(Number(c.env.randomSeed))
        }
        out = env.renderString(c.template, c.context ?? {})
      } else {
        out = renderString(c.template, c.context ?? {})
      }
      assert.equal(out, c.expected)
    },
  )
}
