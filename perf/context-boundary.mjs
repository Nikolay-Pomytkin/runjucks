/**
 * Isolate N-API + JSON context cost vs render work (Runjucks only).
 *
 * Same fixed template; compares mean latency for a minimal context vs a large nested
 * context. The difference is dominated by `serde_json::Value::from_napi_value` work
 * plus any extra cloning proportional to context size — Criterion benches do not include this.
 *
 * Run: `npm run build && npm run perf:context`
 */

import { createRequire } from 'node:module'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { Bench } from 'tinybench'

const __dirname = dirname(fileURLToPath(import.meta.url))
const require = createRequire(import.meta.url)
const { Environment } = require(join(__dirname, '..', 'index.js'))

function buildLargeContext(depth, breadth, leaf = 'x') {
  if (depth <= 0) {
    return leaf
  }
  const o = {}
  for (let i = 0; i < breadth; i++) {
    o[`k${i}`] = buildLargeContext(depth - 1, breadth, leaf)
  }
  return o
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

async function main() {
  const tpl =
    '{{ a }}{{ b }}{{ c }}{% for n in nums %}{{ n }}{% endfor %}{{ x.y.z }}'
  const nums = Array.from({ length: 100 }, (_, i) => i)

  const smallCtx = { a: 1, b: 2, c: 3, nums, x: { y: { z: 'end' } } }
  const largeExtra = buildLargeContext(3, 4)
  const largeCtx = { ...smallCtx, noise: largeExtra }

  const env = new Environment()

  let outSmall
  let outLarge
  try {
    outSmall = env.renderString(tpl, smallCtx)
    outLarge = env.renderString(tpl, largeCtx)
  } catch (e) {
    console.error(e)
    process.exit(1)
  }

  const msSmall = await measureMeanMs('ctx_small', () => {
    env.renderString(tpl, smallCtx)
  })
  const msLarge = await measureMeanMs('ctx_large', () => {
    env.renderString(tpl, largeCtx)
  })

  const json = process.argv.includes('--json')

  if (json) {
    console.log(
      JSON.stringify(
        {
          template_chars: tpl.length,
          small_keys: Object.keys(smallCtx).length,
          large_top_keys: Object.keys(largeCtx).length,
          runjucks_ms_mean: { small: msSmall, large: msLarge },
          delta_ms_mean: msLarge - msSmall,
          outputs_match: outSmall === outLarge,
        },
        null,
        2,
      ),
    )
    return
  }

  console.log('Runjucks context-boundary probe (same template, different context size)\n')
  console.log(`Node ${process.version}`)
  console.log(`Template length: ${tpl.length} chars; loop length: ${nums.length}`)
  console.log('')
  console.log(`  mean ms (small context): ${msSmall.toFixed(4)}`)
  console.log(`  mean ms (large context): ${msLarge.toFixed(4)}`)
  console.log(`  mean delta (large − small): ${(msLarge - msSmall).toFixed(4)} ms`)
  console.log(
    `  (large context includes a deep tree under \`noise\`; template does not read it.)`,
  )
  console.log('')
  console.log(
    'Interpretation: if delta is large relative to small-context time, N-API JSON conversion',
  )
  console.log(
    'and value materialization likely dominate; if delta is tiny, render cost dominates.',
  )
  console.log(`Outputs equal (sanity): ${outSmall === outLarge}`)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
