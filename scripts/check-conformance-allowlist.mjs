#!/usr/bin/env node
/**
 * Ensures every non-skipped conformance fixture id is listed in perf/conformance-allowlist.json
 * so parity.test.mjs and perf/run.mjs stay aligned with the JSON goldens.
 */
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')

function loadJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'))
}

const fixturesDir = join(root, 'native/fixtures/conformance')
const files = [
  ['render_cases', join(fixturesDir, 'render_cases.json')],
  ['filter_cases', join(fixturesDir, 'filter_cases.json')],
  ['tag_parity_cases', join(fixturesDir, 'tag_parity_cases.json')],
]

const allowPath = join(root, 'perf/conformance-allowlist.json')
const allow = loadJson(allowPath)

const allowSet = new Set()
for (const key of ['render_cases', 'filter_cases', 'tag_parity_cases']) {
  for (const id of allow[key] ?? []) {
    allowSet.add(id)
  }
}

const missing = []
const extraInAllowlist = []
const missingDivergenceNote = []
const summary = {
  total: 0,
  skipped: 0,
  comparedWithNunjucks: 0,
  runjucksOnlyGoldens: 0,
  bySuite: {},
}

for (const [suite, file] of files) {
  const cases = loadJson(file)
  if (!Array.isArray(cases)) {
    throw new Error(`Expected array in ${file}`)
  }
  summary.bySuite[suite] = {
    total: 0,
    skipped: 0,
    comparedWithNunjucks: 0,
    runjucksOnlyGoldens: 0,
  }
  for (const c of cases) {
    if (!c.id) continue
    summary.total += 1
    summary.bySuite[suite].total += 1
    if (c.skip === true) {
      summary.skipped += 1
      summary.bySuite[suite].skipped += 1
      continue
    }
    if (c.compareWithNunjucks === false) {
      summary.runjucksOnlyGoldens += 1
      summary.bySuite[suite].runjucksOnlyGoldens += 1
    } else {
      summary.comparedWithNunjucks += 1
      summary.bySuite[suite].comparedWithNunjucks += 1
    }
    if (!allowSet.has(c.id)) {
      missing.push({ suite, file, id: c.id })
    }
    if (c.compareWithNunjucks === false) {
      const note = c.divergenceNote
      if (typeof note !== 'string' || !note.trim()) {
        missingDivergenceNote.push({ suite, file, id: c.id })
      }
    }
  }
}

for (const id of allowSet) {
  let found = false
  for (const [, file] of files) {
    const cases = loadJson(file)
    for (const c of cases) {
      if (c.id === id) {
        found = true
        break
      }
    }
    if (found) break
  }
  if (!found) {
    extraInAllowlist.push(id)
  }
}

let ok = true
if (missing.length) {
  ok = false
  console.error('fixture ids missing from perf/conformance-allowlist.json:\n')
  for (const m of missing) {
    console.error(`  - ${m.id} (${m.suite})`)
  }
}

if (extraInAllowlist.length) {
  ok = false
  console.error('\nallowlist ids with no matching fixture (stale or typo):\n')
  for (const id of extraInAllowlist) {
    console.error(`  - ${id}`)
  }
}

if (missingDivergenceNote.length) {
  ok = false
  console.error(
    '\ncompareWithNunjucks: false requires non-empty divergenceNote:\n',
  )
  for (const m of missingDivergenceNote) {
    console.error(`  - ${m.id} (${m.suite})`)
  }
}

if (!ok) {
  process.exit(1)
}

console.log(
  `check-conformance-allowlist: ok (${allowSet.size} ids in allowlist, all fixtures covered)`,
)
console.log(
  `parity summary: total=${summary.total}, comparedWithNunjucks=${summary.comparedWithNunjucks}, runjucksOnlyGoldens=${summary.runjucksOnlyGoldens}, skipped=${summary.skipped}`,
)
for (const [suite, stats] of Object.entries(summary.bySuite)) {
  console.log(
    `  ${suite}: total=${stats.total}, comparedWithNunjucks=${stats.comparedWithNunjucks}, runjucksOnlyGoldens=${stats.runjucksOnlyGoldens}, skipped=${stats.skipped}`,
  )
}
