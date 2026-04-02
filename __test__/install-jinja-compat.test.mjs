import assert from 'node:assert/strict'
import test from 'node:test'
import { installJinjaCompat } from '../install-jinja-compat.js'

test('installJinjaCompat is a no-op', () => {
  assert.equal(installJinjaCompat(), undefined)
})
