/**
 * Regression gate: runjucks NAPI output must match the reference `nunjucks` npm package
 * for allowlisted fixture IDs (see perf/conformance-allowlist.json).
 */
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

import { conformanceCasesById } from './conformance/load-fixtures.mjs'
import {
  makeRunjucksEnv,
  makeNunjucksEnv,
  nunjucks,
} from '../perf/harness-env.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const pkgRoot = join(__dirname, '..')

const allowlist = JSON.parse(
  readFileSync(join(pkgRoot, 'perf/conformance-allowlist.json'), 'utf8'),
)

function cloneCtx(ctx) {
  return structuredClone(ctx ?? {})
}

const byId = conformanceCasesById()

function collectIds() {
  const ids = []
  for (const key of ['render_cases', 'filter_cases', 'tag_parity_cases']) {
    const list = allowlist[key]
    if (Array.isArray(list)) ids.push(...list)
  }
  return ids
}

for (const id of collectIds()) {
  const c = byId.get(id)
  test(`parity vs nunjucks: ${id}`, { skip: c?.skip === true }, () => {
    assert.ok(c, `unknown allowlist id: ${id}`)
    const tpl = c.template
    const ctx = cloneCtx(c.context)

    const rjEnv = makeRunjucksEnv(c)
    const njEnv = makeNunjucksEnv(c)

    let uninstallJinja = null
    if (c.env?.jinjaCompat === true) {
      uninstallJinja = nunjucks.installJinjaCompat()
    }

    let rOut
    let nOut
    try {
      rOut = rjEnv.renderString(tpl, ctx)
      nOut = njEnv.renderString(tpl, ctx)
    } catch (e) {
      assert.fail(`render error for ${id}: ${e.message}`)
    } finally {
      if (uninstallJinja) uninstallJinja()
    }

    assert.equal(
      rOut,
      nOut,
      `runjucks vs nunjucks mismatch for ${id}`,
    )
    assert.equal(
      rOut,
      c.expected,
      `runjucks vs golden expected for ${id}`,
    )
  })
}
