/**
 * Regression gate: runjucks NAPI output must match the reference `nunjucks` npm package
 * for allowlisted fixture IDs (see perf/conformance-allowlist.json).
 */
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createRequire } from 'node:module'
import test from 'node:test'

import { conformanceCasesById } from './conformance/load-fixtures.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const pkgRoot = join(__dirname, '..')
const require = createRequire(import.meta.url)

const runjucks = require(join(pkgRoot, 'index.js'))
const nunjucks = require('nunjucks')

const allowlist = JSON.parse(
  readFileSync(join(pkgRoot, 'perf/conformance-allowlist.json'), 'utf8'),
)

function applyRunjucksEnvOptions(env, envOpts) {
  if (!envOpts) return
  const ae = envOpts.autoescape
  env.setAutoescape(ae !== false)
  if (envOpts.dev === true) env.setDev(true)
  if (envOpts.randomSeed != null && typeof env.setRandomSeed === 'function') {
    env.setRandomSeed(Number(envOpts.randomSeed))
  }
  if (typeof env.configure === 'function') {
    const configOpts = {}
    if (envOpts.throwOnUndefined === true) configOpts.throwOnUndefined = true
    if (envOpts.trimBlocks === true) configOpts.trimBlocks = true
    if (envOpts.lstripBlocks === true) configOpts.lstripBlocks = true
    if (envOpts.tags) configOpts.tags = envOpts.tags
    if (Object.keys(configOpts).length > 0) env.configure(configOpts)
  }
  if (envOpts.globals) {
    for (const [name, value] of Object.entries(envOpts.globals)) {
      env.addGlobal(name, value)
    }
  }
  if (envOpts.templateMap && typeof env.setTemplateMap === 'function') {
    env.setTemplateMap(envOpts.templateMap)
  }
}

/** Nunjucks has no JSON marker for `is callable`; map runjucks marker objects to a no-op function. */
function valueForNunjucksGlobal(value) {
  if (
    value &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    value.__runjucks_callable === true
  ) {
    return () => {}
  }
  return value
}

function makeNunjucksLoaderFromTemplateMap(map) {
  const TemplateMapLoader = nunjucks.Loader.extend({
    getSource(name) {
      const src = map[name]
      if (src === undefined) {
        return null
      }
      return { src, path: name, noCache: false }
    },
  })
  return new TemplateMapLoader()
}

function makeNunjucksEnv(case_) {
  const e = case_.env
  const autoescape = e?.autoescape !== false
  const dev = e?.dev === true
  const throwOnUndefined = e?.throwOnUndefined === true
  const trimBlocks = e?.trimBlocks === true
  const lstripBlocks = e?.lstripBlocks === true
  const loader =
    e?.templateMap != null
      ? makeNunjucksLoaderFromTemplateMap(e.templateMap)
      : null
  const njOpts = {
    autoescape,
    dev,
    throwOnUndefined,
    trimBlocks,
    lstripBlocks,
  }
  if (e?.tags) njOpts.tags = e.tags
  const env = new nunjucks.Environment(loader, njOpts)
  if (e?.globals) {
    for (const [name, value] of Object.entries(e.globals)) {
      env.addGlobal(name, valueForNunjucksGlobal(value))
    }
  }
  return env
}

function makeRunjucksEnv(case_) {
  const env = new runjucks.Environment()
  applyRunjucksEnvOptions(env, case_.env)
  return env
}

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
