'use strict'

const path = require('node:path')
const { Environment } = require('./index.js')

function resolveViewsDir(raw) {
  if (raw == null) return path.resolve(process.cwd())
  const first = Array.isArray(raw) ? raw[0] : raw
  return path.resolve(String(first))
}

/** Express merges `res.locals` into `viewOpts` and may attach helper functions — only JSON data reaches the Rust renderer. */
function templateContextFromViewOpts(viewOpts) {
  const merged = { ...viewOpts }
  if (viewOpts._locals && typeof viewOpts._locals === 'object') {
    Object.assign(merged, viewOpts._locals)
  }
  delete merged._locals
  try {
    return JSON.parse(JSON.stringify(merged))
  } catch {
    return {}
  }
}

/**
 * Register Runjucks as an Express view engine.
 *
 * @param {import('express').Express} app
 * @param {object} [opts]
 * @param {string | string[]} [opts.views] - Template root directory (absolute or cwd-relative). Defaults to `app.get('views')` when set, otherwise `process.cwd()`. If `views` is an array, the first entry is used (Express allows multiple roots).
 * @param {string} [opts.ext] - Engine extension without dot (default `njk`).
 * @param {import('./index.js').ConfigureOptions} [opts.configure] - Passed to `env.configure()` before `setLoaderRoot`.
 * @param {boolean} [opts.invalidateOnViewCacheOff] - When `true` (default), if `app.get('view cache') === false` (typical in development), call `env.invalidateCache()` before each render so disk-backed templates pick up lexer/parser changes without a stale parse cache. Express’s `view cache` controls Express’s own template path resolution cache; Runjucks additionally caches parsed ASTs — this option aligns “no view cache” with clearing that parse cache.
 * @returns {import('./index.js').Environment}
 */
function expressEngine(app, opts = {}) {
  const fromApp =
    typeof app.get === 'function' && app.get('views') ? app.get('views') : null
  const viewsRoot = resolveViewsDir(opts.views ?? fromApp)
  const ext = opts.ext ?? 'njk'
  const invalidateOnViewCacheOff = opts.invalidateOnViewCacheOff !== false
  const env = new Environment()
  if (opts.configure) {
    env.configure(opts.configure)
  }
  env.setLoaderRoot(viewsRoot)
  app.engine(ext, (filePath, viewOpts, cb) => {
    try {
      if (
        invalidateOnViewCacheOff &&
        typeof app.get === 'function' &&
        app.get('view cache') === false
      ) {
        env.invalidateCache()
      }
      const views =
        typeof app.get === 'function' && app.get('views')
          ? resolveViewsDir(app.get('views'))
          : viewsRoot
      const rel = path.relative(views, filePath).split(path.sep).join('/')
      const ctx = templateContextFromViewOpts(viewOpts)
      const html = env.renderTemplate(rel, ctx)
      cb(null, html)
    } catch (err) {
      cb(err)
    }
  })
  return env
}

module.exports = { expressEngine }
