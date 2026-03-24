'use strict'

/**
 * Minimal sync API resembling nunjucks Environment + Template for string-only
 * templates. Delegates to runjucks native `renderString` / `Environment`.
 *
 * @param {typeof import('../index.js')} runjucks
 */
function createCompat(runjucks) {
  class Environment {
    constructor(_loaders, opts = {}) {
      this._inner = new runjucks.Environment()
      if (opts.autoescape === false) {
        this._inner.setAutoescape(false)
      }
      if (opts.dev === true) {
        this._inner.setDev(true)
      }
    }

    renderString(template, ctx) {
      return runjucks.renderString(template, ctx ?? {})
    }
  }

  class Template {
    /**
     * @param {string} src
     * @param {InstanceType<typeof Environment>} env
     */
    constructor(src, env) {
      this._src = src
      this._env = env
    }

    render(ctx) {
      return this._env.renderString(this._src, ctx ?? {})
    }
  }

  return { Environment, Template }
}

module.exports = { createCompat }
