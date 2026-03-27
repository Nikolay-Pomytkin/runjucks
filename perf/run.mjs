/**
 * Local perf harness: runjucks (NAPI) vs nunjucks npm.
 * Run from package root: `npm run build && npm run perf`
 * Optional: `node perf/run.mjs --json` for machine-readable output.
 * Optional: `node perf/run.mjs --cold` — Runjucks uses a fresh Environment each iteration (cold parse); default reuses one env (warm parsed-template cache).
 */

import { readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { Bench } from 'tinybench'
import { syntheticCases } from './synthetic.mjs'
import { conformanceCasesById } from '../__test__/conformance/load-fixtures.mjs'
import {
  makeRunjucksEnv,
  makeNunjucksEnv,
  nunjucks,
} from './harness-env.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))

const allowlist = JSON.parse(
  readFileSync(join(__dirname, 'conformance-allowlist.json'), 'utf8'),
)

const jsonOut = process.argv.includes('--json')
const coldRunjucks = process.argv.includes('--cold')

function cloneCtx(ctx) {
  return structuredClone(ctx)
}

async function measureMeanMs(label, fn) {
  const bench = new Bench({
    name: label,
    time: 450,
    warmupTime: 120,
  })
  bench.add(label, fn)
  await bench.run()
  const task = bench.getTask(label)
  const state = task?.result?.state
  if (state !== 'completed') {
    throw new Error(`${label}: bench state=${state} ${task?.result?.error ?? ''}`)
  }
  return task.result.latency.mean
}

async function benchCase(case_) {
  const { name, template, context = {}, expected } = case_
  const tpl = template
  const ctx = context

  if (case_.skip === true) {
    return {
      name,
      skip: true,
      reason: 'fixture marked skip (pending parity)',
    }
  }

  if (case_.compareWithNunjucks === false) {
    return {
      name,
      skip: true,
      reason:
        'compareWithNunjucks false (runjucks-only golden; no nunjucks perf baseline)',
    }
  }

  let uninstallJinja = null
  if (case_.env?.jinjaCompat === true) {
    uninstallJinja = nunjucks.installJinjaCompat()
  }

  try {
    const rjEnv = makeRunjucksEnv(case_)
    const njEnv = makeNunjucksEnv(case_)

    let rOut
    let nOut
    try {
      rOut = rjEnv.renderString(tpl, cloneCtx(ctx))
      nOut = njEnv.renderString(tpl, cloneCtx(ctx))
    } catch (e) {
      return {
        name,
        skip: true,
        reason: `render error: ${e.message}`,
      }
    }
    if (rOut !== nOut) {
      return {
        name,
        skip: true,
        reason: 'parity mismatch runjucks vs nunjucks',
      }
    }
    if (expected !== undefined && rOut !== expected) {
      return {
        name,
        skip: true,
        reason: 'output differs from fixture expected (update allowlist or engine)',
      }
    }

    const rjMs = await measureMeanMs(`rj:${name}`, () => {
      const env = coldRunjucks ? makeRunjucksEnv(case_) : rjEnv
      env.renderString(tpl, cloneCtx(ctx))
    })

    const njMs = await measureMeanMs(`nj:${name}`, () => {
      njEnv.renderString(tpl, cloneCtx(ctx))
    })

    const speedup = njMs / rjMs
    return { name, rjMs, njMs, speedup, skip: false }
  } finally {
    if (uninstallJinja) uninstallJinja()
  }
}

function pad(s, n) {
  const str = String(s)
  return str.length >= n ? str.slice(0, n) : str + ' '.repeat(n - str.length)
}

function collectAllowlistedCases(conformanceById) {
  const conformanceCases = []
  for (const id of allowlist.render_cases ?? []) {
    const c = conformanceById.get(id)
    if (!c) {
      console.warn(`allowlist: missing render case id ${id}`)
      continue
    }
    conformanceCases.push({ ...c, name: `conf:${id}` })
  }
  for (const id of allowlist.filter_cases ?? []) {
    const c = conformanceById.get(id)
    if (!c) {
      console.warn(`allowlist: missing filter case id ${id}`)
      continue
    }
    conformanceCases.push({ ...c, name: `conf:${id}` })
  }
  for (const id of allowlist.tag_parity_cases ?? []) {
    const c = conformanceById.get(id)
    if (!c) {
      console.warn(`allowlist: missing tag_parity case id ${id}`)
      continue
    }
    conformanceCases.push({ ...c, name: `conf:${id}` })
  }
  return conformanceCases
}

async function main() {
  if (!jsonOut) {
    console.log('runjucks perf vs nunjucks (local only; noisy across machines)\n')
    console.log(`Node ${process.version} | nunjucks 3.2.4`)
    if (coldRunjucks) {
      console.log('Runjucks: --cold (fresh Environment each iteration)')
    } else {
      console.log('Runjucks: warm (one Environment per case; parsed templates cached)')
    }
    console.log('')
  }

  const conformanceById = conformanceCasesById()
  const conformanceCases = collectAllowlistedCases(conformanceById)

  const all = [
    ...syntheticCases().map((c) => ({ ...c, expected: undefined })),
    ...conformanceCases,
  ]

  const rows = []
  for (const c of all) {
    const row = await benchCase(c)
    rows.push(row)
    if (!jsonOut && row.skip) {
      console.log(`SKIP ${pad(row.name, 36)} ${row.reason}`)
    }
  }

  const ok = rows.filter((r) => !r.skip)
  const avgSp =
    ok.length > 0 ? ok.reduce((a, r) => a + r.speedup, 0) / ok.length : 0

  if (jsonOut) {
    const payload = {
      node: process.version,
      generatedAt: new Date().toISOString(),
      rows: rows.map((r) =>
        r.skip
          ? { name: r.name, skip: true, reason: r.reason }
          : {
              name: r.name,
              skip: false,
              runjucks_ms: r.rjMs,
              nunjucks_ms: r.njMs,
              nj_over_rj: r.speedup,
            },
      ),
      summary: {
        nonSkippedCount: ok.length,
        avg_nj_over_rj: avgSp || null,
      },
    }
    const outPath = join(__dirname, 'last-run.json')
    writeFileSync(outPath, JSON.stringify(payload, null, 2), 'utf8')
    console.log(`Wrote ${outPath}`)
    return
  }

  console.log('')
  console.log(
    `${pad('case', 38)} ${pad('runjucks_ms', 12)} ${pad('nunjucks_ms', 12)} ${pad('nj/rj', 8)}`,
  )
  console.log('-'.repeat(74))

  for (const row of rows) {
    if (row.skip) continue
    console.log(
      `${pad(row.name, 38)} ${pad(row.rjMs.toFixed(4), 12)} ${pad(row.njMs.toFixed(4), 12)} ${pad(row.speedup.toFixed(2) + 'x', 8)}`,
    )
  }

  if (ok.length) {
    console.log('-'.repeat(74))
    console.log(
      `${pad('avg nj/rj (non-skipped)', 38)} ${pad('', 12)} ${pad('', 12)} ${pad(avgSp.toFixed(2) + 'x', 8)}`,
    )
    console.log(
      '\n>1x means nunjucks is slower (runjucks faster). <1x means runjucks slower.',
    )
  }
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
