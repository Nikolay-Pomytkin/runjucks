/**
 * Local perf harness: runjucks (NAPI) vs nunjucks npm.
 * Run from package root: `npm run build && npm run perf`
 */

import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createRequire } from 'node:module'
import { Bench } from 'tinybench'
import { syntheticCases } from './synthetic.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const pkgRoot = join(__dirname, '..')
const require = createRequire(import.meta.url)

const runjucks = require(join(pkgRoot, 'index.js'))
const nunjucks = require('nunjucks')

const allowlist = JSON.parse(
  readFileSync(join(__dirname, 'conformance-allowlist.json'), 'utf8'),
)

function loadConformanceFiles() {
  const renderPath = join(
    pkgRoot,
    'native/fixtures/conformance/render_cases.json',
  )
  const filterPath = join(
    pkgRoot,
    'native/fixtures/conformance/filter_cases.json',
  )
  const render = JSON.parse(readFileSync(renderPath, 'utf8'))
  const filter = JSON.parse(readFileSync(filterPath, 'utf8'))
  const byId = new Map()
  for (const c of render) {
    byId.set(c.id, { ...c, _file: 'render_cases' })
  }
  for (const c of filter) {
    byId.set(c.id, { ...c, _file: 'filter_cases' })
  }
  return byId
}

function makeRunjucksEnv(case_) {
  const env = new runjucks.Environment()
  const ae = case_.env?.autoescape
  env.setAutoescape(ae !== false)
  return env
}

function makeNunjucksEnv(case_) {
  const autoescape = case_.env?.autoescape !== false
  return new nunjucks.Environment(null, { autoescape })
}

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
    rjEnv.renderString(tpl, cloneCtx(ctx))
  })

  const njMs = await measureMeanMs(`nj:${name}`, () => {
    njEnv.renderString(tpl, cloneCtx(ctx))
  })

  const speedup = njMs / rjMs
  return { name, rjMs, njMs, speedup, skip: false }
}

function pad(s, n) {
  const str = String(s)
  return str.length >= n ? str.slice(0, n) : str + ' '.repeat(n - str.length)
}

async function main() {
  console.log('runjucks perf vs nunjucks (local only; noisy across machines)\n')
  console.log(`Node ${process.version} | nunjucks 3.2.4\n`)

  const conformanceById = loadConformanceFiles()
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

  const all = [
    ...syntheticCases().map((c) => ({ ...c, expected: undefined })),
    ...conformanceCases,
  ]

  const rows = []
  for (const c of all) {
    const row = await benchCase(c)
    rows.push(row)
    if (row.skip) {
      console.log(`SKIP ${pad(row.name, 36)} ${row.reason}`)
    }
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

  const ok = rows.filter((r) => !r.skip)
  if (ok.length) {
    const avgSp = ok.reduce((a, r) => a + r.speedup, 0) / ok.length
    console.log('-'.repeat(74))
    console.log(
      `${pad('avg nj/rj (non-skipped)', 38)} ${pad('', 12)} ${pad('', 12)} ${pad(avgSp.toFixed(2) + 'x', 8)}`,
    )
    console.log('\n>1x means nunjucks is slower (runjucks faster). <1x means runjucks slower.')
  }
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
