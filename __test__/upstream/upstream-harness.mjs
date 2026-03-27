/**
 * Shared helpers for tests ported from vendored Nunjucks (`nunjucks/tests/*.js`).
 * Mirrors `tests/util.js` defaults: `dev: true`, sync `renderString`-style rendering.
 */
import assert from 'node:assert/strict'
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

import { createRequire } from 'node:module'

const __dirname = dirname(fileURLToPath(import.meta.url))
const pkgRoot = join(__dirname, '..', '..')
const require = createRequire(import.meta.url)
const runjucks = require(join(pkgRoot, 'index.js'))

export { runjucks }

/** @type {import('nunjucks') | null} */
let nunjucks = null
try {
  nunjucks = require('nunjucks')
} catch {
  nunjucks = null
}

/**
 * Root of vendored `nunjucks/tests` (contains `templates/`, `util.js`, …).
 * Set `RUNJUCKS_NUNJUCKS_TESTS` to override when the repo layout differs.
 */
export function resolveNunjucksTestsRoot() {
  const env = process.env.RUNJUCKS_NUNJUCKS_TESTS
  if (env && existsSync(env)) return env
  const fromUpstream = join(__dirname, '..', '..', '..', 'nunjucks', 'tests')
  if (existsSync(fromUpstream)) return fromUpstream
  return null
}

/**
 * Recursively build a `name → source` map like `Environment#setTemplateMap` expects.
 * Keys are POSIX-style paths relative to `rootDir` (e.g. `relative/include.njk`).
 *
 * @param {string} rootDir Absolute directory to walk
 * @param {string} [baseRel] Internal use
 * @returns {Record<string, string>}
 */
export function buildTemplateMapFromDir(rootDir, baseRel = '') {
  /** @type {Record<string, string>} */
  const map = {}
  const entries = readdirSync(rootDir, { withFileTypes: true })
  for (const ent of entries) {
    const rel = join(baseRel, ent.name)
    const full = join(rootDir, ent.name)
    if (ent.isDirectory()) {
      Object.assign(map, buildTemplateMapFromDir(full, rel))
    } else if (ent.isFile()) {
      const key = rel.split('\\').join('/')
      map[key] = readFileSync(full, 'utf8')
    }
  }
  return map
}

/**
 * @param {string} [subDir] Relative to `nunjucks/tests`, default `templates`
 */
export function templateMapFromNunjucksTests(subDir = 'templates') {
  const root = resolveNunjucksTestsRoot()
  if (!root) return null
  const dir = join(root, subDir)
  if (!existsSync(dir)) return null
  return buildTemplateMapFromDir(dir)
}

/**
 * Default upstream env: matches Nunjucks `util.render` (`opts.dev = true`).
 * @param {object} [opts]
 * @param {boolean} [opts.autoescape] default `true`
 * @param {boolean} [opts.throwOnUndefined]
 * @param {boolean} [opts.trimBlocks]
 * @param {boolean} [opts.lstripBlocks]
 */
export function createUpstreamEnvironment(opts = {}) {
  const env = new runjucks.Environment()
  env.setDev(true)
  if (opts.autoescape === false) env.setAutoescape(false)
  const cfg = {}
  if (opts.throwOnUndefined === true) cfg.throwOnUndefined = true
  if (opts.trimBlocks === true) cfg.trimBlocks = true
  if (opts.lstripBlocks === true) cfg.lstripBlocks = true
  if (Object.keys(cfg).length) env.configure(cfg)
  if (opts.templateMap && typeof env.setTemplateMap === 'function') {
    env.setTemplateMap(opts.templateMap)
  }
  return env
}

/**
 * @param {import('../index.js').Environment} env
 * @param {string} template
 * @param {Record<string, unknown>} [context]
 */
export function renderUpstream(env, template, context = {}) {
  return env.renderString(template, context)
}

/**
 * Assert `actual === expected` with a clear message (like Nunjucks `equal`).
 */
export function assertRendered(assertMod, actual, expected, label = '') {
  assertMod.equal(
    actual,
    expected,
    label ? `${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}` : undefined,
  )
}

/**
 * Optional: compare Runjucks output to the reference `nunjucks` npm package (same template + env shape).
 * Use when validating a newly ported case; not required for CI if goldens are trusted.
 *
 * @param {object} params
 * @param {string} params.template
 * @param {Record<string, unknown>} [params.context]
 * @param {object} [params.nunjucksEnvOpts] Options for `nunjucks.Environment` (loader, autoescape, …)
 * @param {string} params.runjucksOut
 */
export function assertMatchesNunjucksReference({
  template,
  context = {},
  nunjucksEnvOpts = {},
  runjucksOut,
}) {
  if (!nunjucks) {
    return
  }
  const loader =
    nunjucksEnvOpts.templateMap != null
      ? nunjucks.Loader.extend({
          getSource(name) {
            const src = nunjucksEnvOpts.templateMap[name]
            if (src === undefined) return null
            return { src, path: name, noCache: false }
          },
        })
      : null
  const nj = new nunjucks.Environment(loader, {
    autoescape: nunjucksEnvOpts.autoescape !== false,
    dev: true,
    throwOnUndefined: nunjucksEnvOpts.throwOnUndefined === true,
    trimBlocks: nunjucksEnvOpts.trimBlocks === true,
    lstripBlocks: nunjucksEnvOpts.lstripBlocks === true,
  })
  const t = new nunjucks.Template(template, nj)
  const nOut = t.render(context)
  assert.equal(
    runjucksOut,
    nOut,
    'runjucks output should match nunjucks npm for this ported case',
  )
}

export { assert }
