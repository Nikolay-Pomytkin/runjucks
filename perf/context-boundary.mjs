/**
 * Isolate N-API + JSON context cost vs render work (Runjucks only).
 *
 * Same fixed template; compares mean latency for:
 * - **object** context (`renderString`) — full JS object graph → `serde_json::Value` via napi-rs
 * - **JSON string** (`renderStringFromJson`) — one JS string, Rust parse (`simd-json` by default in `runjucks-napi`)
 * - **JSON bytes** (`renderStringFromJsonBuffer`) — `Buffer` / UTF-8 bytes; skips extra Rust `String` wrapper for the payload
 *
 * Run: `npm run build && npm run perf:context`
 */
import { Buffer } from 'node:buffer'
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
  const largeJsonStr = JSON.stringify(largeCtx)
  const largeJsonBuf = Buffer.from(largeJsonStr, 'utf8')

  const env = new Environment()

  let outSmall
  let outLarge
  let outJsonStr
  let outJsonBuf
  try {
    outSmall = env.renderString(tpl, smallCtx)
    outLarge = env.renderString(tpl, largeCtx)
    outJsonStr = env.renderStringFromJson(tpl, largeJsonStr)
    outJsonBuf = env.renderStringFromJsonBuffer(tpl, largeJsonBuf)
  } catch (e) {
    console.error(e)
    process.exit(1)
  }

  const msSmallObj = await measureMeanMs('object_small', () => {
    env.renderString(tpl, smallCtx)
  })
  const msLargeObj = await measureMeanMs('object_large', () => {
    env.renderString(tpl, largeCtx)
  })
  const msLargeJsonStr = await measureMeanMs('from_json_string_large', () => {
    env.renderStringFromJson(tpl, largeJsonStr)
  })
  const msLargeJsonBuf = await measureMeanMs('from_json_buffer_large', () => {
    env.renderStringFromJsonBuffer(tpl, largeJsonBuf)
  })

  const json = process.argv.includes('--json')

  const deltaObj = msLargeObj - msSmallObj
  const speedupJsonStrVsObjLarge =
    msLargeObj > 0 ? msLargeObj / msLargeJsonStr : null
  const speedupJsonBufVsObjLarge =
    msLargeObj > 0 ? msLargeObj / msLargeJsonBuf : null

  if (json) {
    console.log(
      JSON.stringify(
        {
          template_chars: tpl.length,
          small_keys: Object.keys(smallCtx).length,
          large_top_keys: Object.keys(largeCtx).length,
          large_json_bytes: largeJsonBuf.length,
          runjucks_ms_mean: {
            object_small: msSmallObj,
            object_large: msLargeObj,
            from_json_string_large: msLargeJsonStr,
            from_json_buffer_large: msLargeJsonBuf,
          },
          delta_ms_mean: {
            object_large_minus_small: deltaObj,
          },
          speedup_vs_object_large: {
            from_json_string: speedupJsonStrVsObjLarge,
            from_json_buffer: speedupJsonBufVsObjLarge,
          },
          outputs_match:
            outSmall === outLarge &&
            outLarge === outJsonStr &&
            outJsonStr === outJsonBuf,
        },
        null,
        2,
      ),
    )
    return
  }

  console.log('Runjucks context-boundary probe (same template, different context ingress)\n')
  console.log(`Node ${process.version}`)
  console.log(`Template length: ${tpl.length} chars; loop length: ${nums.length}`)
  console.log(`Large JSON UTF-8 size: ${largeJsonBuf.length} bytes`)
  console.log('')
  console.log(`  mean ms (small context, object): ${msSmallObj.toFixed(4)}`)
  console.log(`  mean ms (large context, object):      ${msLargeObj.toFixed(4)}`)
  console.log(`  mean ms (large context, JSON string): ${msLargeJsonStr.toFixed(4)}`)
  console.log(`  mean ms (large context, JSON Buffer):  ${msLargeJsonBuf.toFixed(4)}`)
  console.log(`  Δ object large − small:               ${deltaObj.toFixed(4)} ms`)
  console.log(
    `  speedup vs object large: JSON string ×${speedupJsonStrVsObjLarge?.toFixed(2) ?? 'n/a'}, Buffer ×${speedupJsonBufVsObjLarge?.toFixed(2) ?? 'n/a'}`,
  )
  console.log('')
  console.log(
    'Large context includes a deep tree under `noise`; the template does not read it — extra keys still cross the FFI boundary for the object path.',
  )
  console.log('')
  console.log(`Outputs equal (sanity): ${outSmall === outLarge && outLarge === outJsonStr && outJsonStr === outJsonBuf}`)
}

main().catch((e) => {
  console.error(e)
  process.exit(1)
})
