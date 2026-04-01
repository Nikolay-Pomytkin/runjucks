import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import express from 'express'
import request from 'supertest'
import { expressEngine } from '../express.js'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

test('expressEngine renders a view from the views directory', async () => {
  const views = path.join(__dirname, `.express-fixtures-${process.pid}`)
  fs.mkdirSync(views, { recursive: true })
  fs.writeFileSync(path.join(views, 'hello.njk'), 'Hi {{ name }}', 'utf8')
  try {
    const app = express()
    app.set('views', views)
    app.set('view engine', 'njk')
    expressEngine(app, { ext: 'njk' })
    app.get('/', (req, res) => {
      res.render('hello', { name: 'Ada' })
    })
    const res = await request(app).get('/')
    assert.equal(res.status, 200)
    assert.equal(res.text, 'Hi Ada')
  } finally {
    fs.rmSync(views, { recursive: true, force: true })
  }
})

test('expressEngine passes template errors to Express (missing view)', async () => {
  const views = path.join(__dirname, `.express-missing-${process.pid}`)
  fs.mkdirSync(views, { recursive: true })
  try {
    const app = express()
    app.set('views', views)
    app.set('view engine', 'njk')
    expressEngine(app, { ext: 'njk' })
    app.get('/', (req, res) => {
      res.render('does-not-exist', {})
    })
    const res = await request(app).get('/')
    assert.equal(res.status, 500)
  } finally {
    fs.rmSync(views, { recursive: true, force: true })
  }
})

test('expressEngine opts.configure trimBlocks affects output', async () => {
  const views = path.join(__dirname, `.express-trim-${process.pid}`)
  fs.mkdirSync(views, { recursive: true })
  fs.writeFileSync(
    path.join(views, 't.njk'),
    '{% if true %}\nYES\n{% endif %}',
    'utf8',
  )
  try {
    const app = express()
    app.set('views', views)
    app.set('view engine', 'njk')
    expressEngine(app, {
      ext: 'njk',
      configure: { trimBlocks: true },
    })
    app.get('/', (req, res) => {
      res.render('t', {})
    })
    const res = await request(app).get('/')
    assert.equal(res.status, 200)
    assert.ok(!res.text.includes('\n\n'), 'trimBlocks should reduce blank lines')
    assert.ok(res.text.includes('YES'))
  } finally {
    fs.rmSync(views, { recursive: true, force: true })
  }
})

test('view cache off + invalidateOnViewCacheOff picks up disk template changes', async () => {
  const views = path.join(__dirname, `.express-inval-${process.pid}`)
  fs.mkdirSync(views, { recursive: true })
  const fp = path.join(views, 'live.njk')
  fs.writeFileSync(fp, 'v1', 'utf8')
  try {
    const app = express()
    app.set('views', views)
    app.set('view engine', 'njk')
    app.set('view cache', false)
    expressEngine(app, { ext: 'njk', invalidateOnViewCacheOff: true })
    app.get('/', (req, res) => {
      res.render('live', {})
    })
    const r1 = await request(app).get('/')
    assert.equal(r1.text, 'v1')
    fs.writeFileSync(fp, 'v2', 'utf8')
    const r2 = await request(app).get('/')
    assert.equal(r2.text, 'v2')
  } finally {
    fs.rmSync(views, { recursive: true, force: true })
  }
})
