'use strict'

/**
 * Fetch remote template sources and return a name → source map for {@link Environment#setTemplateMap}
 * or for use with {@link Environment#setLoaderCallback}.
 *
 * Runjucks has no built-in HTTP(S) loader in the native layer (render stays synchronous). Load sources
 * with `fetch`, then register them on the environment — same pattern as Nunjucks “WebLoader” in the
 * browser, adapted for Node.
 *
 * @param {{ name: string, url: string }[]} entries - Template logical names and URLs to fetch.
 * @returns {Promise<Record<string, string>>}
 */
async function fetchTemplateMap(entries) {
  const out = {}
  for (const row of entries) {
    const name = row?.name
    const url = row?.url
    if (typeof name !== 'string' || typeof url !== 'string') {
      throw new TypeError('fetchTemplateMap: each entry needs { name: string, url: string }')
    }
    const res = await fetch(url)
    if (!res.ok) {
      throw new Error(`fetchTemplateMap: ${url} failed with HTTP ${res.status}`)
    }
    out[name] = await res.text()
  }
  return out
}

module.exports = { fetchTemplateMap }
