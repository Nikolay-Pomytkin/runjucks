/**
 * Async render throughput: **Runjucks only** (`renderStringAsync` / `renderTemplateAsync`).
 * There is no Nunjucks baseline — upstream async APIs differ from `renderString`.
 *
 * Usage: `npm run build && npm run perf:async`  (optional `--json` → `perf/last-run-async.json`)
 */

import { readFileSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { Bench } from 'tinybench'
import { applyRunjucksEnvOptions } from './harness-env.mjs'
import { asyncSyntheticCases, asyncSyncParityCases } from './synthetic.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const pkgRoot = join(__dirname, '..')
const require = createRequire(import.meta.url)
const runjucks = require(join(pkgRoot, 'index.js'))

const jsonOut = process.argv.includes('--json')

function readRunjucksVersion() {
  const pkgPath = join(pkgRoot, 'package.json')
  return JSON.parse(readFileSync(pkgPath, 'utf8')).version
}

function cloneCtx(ctx) {
  return structuredClone(ctx ?? {})
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

function pad(s, n) {
  const str = String(s)
  return str.length >= n ? str.slice(0, n) : str + ' '.repeat(n - str.length)
}

async function main() {
  const cases = asyncSyntheticCases()
  const rows = []

  for (const c of cases) {
    const env = new runjucks.Environment()
    applyRunjucksEnvOptions(env, c.env)
    const ctx = cloneCtx(c.context)

    if (c.renderMode === 'template') {
      const nm = c.templateName
      if (typeof nm !== 'string' || !nm) {
        throw new Error(`case ${c.name}: missing templateName`)
      }
      const rjMs = await measureMeanMs(`rj-async:${c.name}`, async () => {
        await env.renderTemplateAsync(nm, cloneCtx(ctx))
      })
      rows.push({ name: c.name, rjMs, kind: 'async_template' })
    } else {
      const tpl = c.template
      if (typeof tpl !== 'string') {
        throw new Error(`case ${c.name}: missing template`)
      }
      const rjMs = await measureMeanMs(`rj-async:${c.name}`, async () => {
        await env.renderStringAsync(tpl, cloneCtx(ctx))
      })
      rows.push({ name: c.name, rjMs, kind: 'async_string' })
    }
  }

  for (const c of asyncSyncParityCases()) {
    const tpl = c.template
    const ctx = cloneCtx(c.context)
    const syncMs = await measureMeanMs(`rj-sync:${c.name}`, () => {
      const env = new runjucks.Environment()
      env.renderString(tpl, cloneCtx(ctx))
    })
    const asyncMs = await measureMeanMs(`rj-async:${c.name}`, async () => {
      const env = new runjucks.Environment()
      await env.renderStringAsync(tpl, cloneCtx(ctx))
    })
    rows.push({
      name: `${c.name}_sync`,
      rjMs: syncMs,
      kind: 'sync_compare',
    })
    rows.push({
      name: `${c.name}_async`,
      rjMs: asyncMs,
      kind: 'async_compare',
    })
    rows.push({
      name: `${c.name}_async_over_sync`,
      ratio: asyncMs / syncMs,
      kind: 'ratio',
    })
  }

  if (jsonOut) {
    const outPath = join(__dirname, 'last-run-async.json')
    const payload = {
      runjucksVersion: readRunjucksVersion(),
      mode: 'async_only',
      node: process.version,
      platform: { platform: process.platform, arch: process.arch },
      generatedAt: new Date().toISOString(),
      rows,
    }
    writeFileSync(outPath, JSON.stringify(payload, null, 2), 'utf8')
    console.log(`Wrote ${outPath}`)
    return
  }

  console.log('runjucks async perf (no Nunjucks baseline)\n')
  console.log(
    `Node ${process.version} | @zneep/runjucks ${readRunjucksVersion()}\n`,
  )
  console.log(`${pad('case', 42)} ${pad('ms_mean', 12)} ${pad('notes', 20)}`)
  console.log('-'.repeat(74))

  for (const r of rows) {
    if (r.kind === 'ratio') {
      console.log(
        `${pad(r.name, 42)} ${pad(r.ratio.toFixed(2) + 'x', 12)} ${pad('async/sync', 20)}`,
      )
    } else {
      const note = r.kind ?? ''
      console.log(
        `${pad(r.name, 42)} ${pad(r.rjMs.toFixed(4), 12)} ${pad(note, 20)}`,
      )
    }
  }
  console.log('\n`ratio` rows: async time vs sync time for the same `for` template (overhead hint).')
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
