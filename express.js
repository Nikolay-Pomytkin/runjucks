'use strict'

const fs = require('node:fs')
const path = require('node:path')
const { Environment } = require('./index.js')

function resolveViewsDir(raw) {
  if (raw == null) return path.resolve(process.cwd())
  const first = Array.isArray(raw) ? raw[0] : raw
  return path.resolve(String(first))
}

/** @param {string | string[] | null | undefined} raw */
function listViewsRoots(raw) {
  if (raw == null) return [path.resolve(process.cwd())]
  if (Array.isArray(raw)) {
    const mapped = raw.map((r) => path.resolve(String(r)))
    return mapped.length > 0 ? mapped : [path.resolve(process.cwd())]
  }
  return [path.resolve(String(raw))]
}

/**
 * @param {string} filePath
 * @param {string[]} roots absolute
 */
function templateNameRelativeToViewsRoots(filePath, roots) {
  const absFile = path.resolve(filePath)
  for (const root of roots) {
    const rel = path.relative(root, absFile)
    if (rel && !rel.startsWith('..') && !path.isAbsolute(rel)) {
      return rel.split(path.sep).join('/')
    }
  }
  return path.relative(roots[0], absFile).split(path.sep).join('/')
}

/**
 * @param {string[]} roots absolute
 */
function createMultiRootLoader(roots) {
  return (name) => {
    const n = String(name).replace(/\\/g, '/')
    for (const root of roots) {
      const full = path.join(root, ...n.split('/'))
      try {
        if (fs.existsSync(full) && fs.statSync(full).isFile()) {
          return fs.readFileSync(full, 'utf8')
        }
      } catch {
        // try next root
      }
    }
    return null
  }
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
 * @param {string | string[]} [opts.views] - Template root directory (absolute or cwd-relative). Defaults to `app.get('views')` when set, otherwise `process.cwd()`. If `views` is an **array**, all roots are searched for template sources (via `setLoaderCallback`); relative names passed to `renderTemplate` are resolved against the root that contains the file Express resolved.
 * @param {string} [opts.ext] - Engine extension without dot (default `njk`).
 * @param {import('./index.js').ConfigureOptions} [opts.configure] - Passed to `env.configure()` before `setLoaderRoot`.
 * @param {boolean} [opts.invalidateOnViewCacheOff] - When `true` (default), if `app.get('view cache') === false` (typical in development), call `env.invalidateCache()` before each render so disk-backed templates pick up lexer/parser changes without a stale parse cache. Express’s `view cache` controls Express’s own template path resolution cache; Runjucks additionally caches parsed ASTs — this option aligns “no view cache” with clearing that parse cache.
 * @returns {import('./index.js').Environment}
 */
function expressEngine(app, opts = {}) {
  const fromApp =
    typeof app.get === 'function' && app.get('views') ? app.get('views') : null
  const rawViews = opts.views ?? fromApp
  const roots = listViewsRoots(rawViews)
  const viewsRoot = roots[0]
  const ext = opts.ext ?? 'njk'
  const invalidateOnViewCacheOff = opts.invalidateOnViewCacheOff !== false
  const env = new Environment()
  if (opts.configure) {
    env.configure(opts.configure)
  }
  if (roots.length > 1) {
    env.setLoaderCallback(createMultiRootLoader(roots))
  } else {
    env.setLoaderRoot(viewsRoot)
  }
  app.engine(ext, (filePath, viewOpts, cb) => {
    try {
      if (
        invalidateOnViewCacheOff &&
        typeof app.get === 'function' &&
        app.get('view cache') === false
      ) {
        env.invalidateCache()
      }
      const viewsRaw =
        typeof app.get === 'function' && app.get('views')
          ? app.get('views')
          : rawViews
      const relRoots = listViewsRoots(viewsRaw)
      const rel = templateNameRelativeToViewsRoots(filePath, relRoots)
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
