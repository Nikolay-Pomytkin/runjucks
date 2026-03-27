/**
 * Shared loader for JSON conformance fixtures (same files as Rust `include_str!` tests).
 * Paths are relative to the runjucks package root.
 */
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
export const FIXTURES_ROOT = join(__dirname, '../../native/fixtures/conformance')

/**
 * @typedef {object} ConformanceCase
 * @property {string} id
 * @property {string} [source]
 * @property {string} template
 * @property {Record<string, unknown>} [context]
 * @property {{ autoescape?: boolean, dev?: boolean, jinjaCompat?: boolean, randomSeed?: number, throwOnUndefined?: boolean, globals?: Record<string, unknown>, templateMap?: Record<string, string> }} [env]
 * @property {string} expected
 * @property {boolean} [skip] — when true, Rust and Node skip until implemented
 * @property {boolean} [compareWithNunjucks] — default true; if false, [`parity.test.mjs`](../parity.test.mjs) only checks runjucks vs `expected` (requires `divergenceNote`).
 * @property {string} [divergenceNote] — required when `compareWithNunjucks === false`; why this case is not compared to nunjucks 3.2.4.
 */

/**
 * @param {string} file - basename under fixtures/conformance, e.g. `render_cases.json`
 * @param {string} suite - logical suite name for metadata
 * @returns {Array<ConformanceCase & { _suite: string }>}
 */
export function loadFixtureFile(file, suite) {
  const path = join(FIXTURES_ROOT, file)
  const raw = JSON.parse(readFileSync(path, 'utf8'))
  if (!Array.isArray(raw)) {
    throw new Error(`Expected array in ${path}`)
  }
  return raw.map((c) => ({ ...c, _suite: suite }))
}

/**
 * All conformance vectors: render + filter + tag parity (same as `cargo test` coverage).
 * @returns {Array<ConformanceCase & { _suite: string }>}
 */
export function loadAllConformanceCases() {
  return [
    ...loadFixtureFile('render_cases.json', 'render_cases'),
    ...loadFixtureFile('filter_cases.json', 'filter_cases'),
    ...loadFixtureFile('tag_parity_cases.json', 'tag_parity_cases'),
  ]
}

/**
 * Map by case id (last write wins if duplicate ids across files — should not happen).
 * @returns {Map<string, ConformanceCase & { _suite: string }>}
 */
export function conformanceCasesById() {
  const m = new Map()
  for (const c of loadAllConformanceCases()) {
    m.set(c.id, c)
  }
  return m
}
