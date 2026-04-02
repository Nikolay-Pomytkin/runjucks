import assert from 'node:assert/strict'
import test from 'node:test'
import { Environment } from '../index.js'
import {
  renderWithCallback,
  renderWithCallbackAsync,
} from '../render-with-callback.js'

test('renderWithCallback success and error paths', () => {
  const env = new Environment()
  env.setTemplateMap({ ok: 'x{{ n }}', bad: '{{ nope }}' })

  renderWithCallback(env, 'ok', { n: 1 }, (err, html) => {
    assert.equal(err, null)
    assert.equal(html, 'x1')
  })

  renderWithCallback(env, 'bad', {}, (err) => {
    assert.ok(err instanceof Error)
  })

  assert.throws(
    () => renderWithCallback(env, 'ok', {}, null),
    /cb must be a function/,
  )
})

test('renderWithCallbackAsync success and error paths', async () => {
  const env = new Environment()
  env.setTemplateMap({ ok: 'a{{ n }}' })

  await new Promise((resolve, reject) => {
    renderWithCallbackAsync(env, 'ok', { n: 2 }, (err, html) => {
      try {
        assert.equal(err, null)
        assert.equal(html, 'a2')
        resolve()
      } catch (e) {
        reject(e)
      }
    })
  })

  await new Promise((resolve, reject) => {
    renderWithCallbackAsync(env, 'not-in-map', {}, (err) => {
      try {
        assert.ok(err instanceof Error)
        resolve()
      } catch (e) {
        reject(e)
      }
    })
  })

  assert.throws(
    () => renderWithCallbackAsync(env, 'ok', {}, null),
    /cb must be a function/,
  )
})
