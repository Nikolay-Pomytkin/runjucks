'use strict'

/**
 * Convert a value to JSON-serializable data for `renderString` / `renderTemplate` context.
 * `Map` becomes a plain object (string keys); `Set` becomes an array. Nested values use the same replacer.
 *
 * @param {unknown} value
 * @returns {unknown}
 */
function serializeContextForRender(value) {
  return JSON.parse(
    JSON.stringify(value, (_k, v) => {
      if (v instanceof Map) return Object.fromEntries(v)
      if (v instanceof Set) return [...v]
      return v
    }),
  )
}

module.exports = { serializeContextForRender }
