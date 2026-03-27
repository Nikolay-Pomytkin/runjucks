/**
 * Runjucks-only render errors (substrings). Not compared to nunjucks messages.
 * Fixtures: native/fixtures/conformance/error_cases.json
 */
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import test from 'node:test'

import { Environment } from '../index.js'

const __dirname = dirname(fileURLToPath(import.meta.url))
const root = join(__dirname, '../native/fixtures/conformance')
const cases = JSON.parse(readFileSync(join(root, 'error_cases.json'), 'utf8'))

for (const c of cases) {
  test(`error case ${c.id}`, () => {
    const env = new Environment()
    assert.throws(
      () => {
        env.renderString(c.template, c.context ?? {})
      },
      (err) => {
        assert.ok(
          err && typeof err.message === 'string',
          'throws with message',
        )
        assert.ok(
          err.message.includes(c.errorContains),
          `expected message to include ${JSON.stringify(c.errorContains)}, got ${JSON.stringify(err.message)}`,
        )
        return true
      },
    )
  })
}
