'use strict'

/**
 * Invoke Nunjucks-style `(err, html)` callbacks without implementing a
 * callback-only core API. Sync path uses `renderTemplate`; async path uses
 * `renderTemplateAsync` and forwards the Promise result to the callback.
 *
 * @param {import('./index.js').Environment} env
 * @param {string} name
 * @param {unknown} ctx
 * @param {(err: Error | null, html?: string) => void} cb
 */
function renderWithCallback(env, name, ctx, cb) {
  if (typeof cb !== 'function') {
    throw new TypeError('cb must be a function')
  }
  try {
    const html = env.renderTemplate(name, ctx)
    cb(null, html)
  } catch (err) {
    cb(err)
  }
}

/**
 * @param {import('./index.js').Environment} env
 * @param {string} name
 * @param {unknown} ctx
 * @param {(err: Error | null, html?: string) => void} cb
 */
function renderWithCallbackAsync(env, name, ctx, cb) {
  if (typeof cb !== 'function') {
    throw new TypeError('cb must be a function')
  }
  env.renderTemplateAsync(name, ctx).then(
    (html) => cb(null, html),
    (err) => cb(err),
  )
}

module.exports = { renderWithCallback, renderWithCallbackAsync }
