import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import test from 'node:test'
import { fileURLToPath } from 'node:url'
import express from 'express'
import request from 'supertest'
import { expressEngine } from '../express.js'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

test('expressEngine JSON-roundtrips merged locals; functions are stripped', async () => {
  const views = path.join(__dirname, `.express-locals-${process.pid}`)
  fs.mkdirSync(views, { recursive: true })
  fs.writeFileSync(path.join(views, 'hi.njk'), '{{ x }} {{ y.z }}', 'utf8')
  try {
    const app = express()
    app.set('views', views)
    app.set('view engine', 'njk')
    expressEngine(app, { ext: 'njk' })
    app.get('/', (req, res) => {
      res.locals.fn = () => 'should-not-appear'
      res.locals.x = 2
      res.render('hi', { y: { z: 3 } })
    })
    const res = await request(app).get('/')
    assert.equal(res.status, 200)
    assert.equal(res.text.trim(), '2 3')
  } finally {
    fs.rmSync(views, { recursive: true, force: true })
  }
})

test('expressEngine forwards render errors to Express error middleware', async () => {
  const views = path.join(__dirname, `.express-errmw-${process.pid}`)
  fs.mkdirSync(views, { recursive: true })
  try {
    const app = express()
    app.set('views', views)
    app.set('view engine', 'njk')
    expressEngine(app, { ext: 'njk' })
    app.get('/', (req, res) => {
      res.render('missing-view', {})
    })
    const captured = []
    app.use((err, req, res, next) => {
      captured.push(err)
      res.status(500).send('handled')
    })
    const res = await request(app).get('/')
    assert.equal(res.status, 500)
    assert.equal(res.text, 'handled')
    assert.equal(captured.length, 1)
    assert.ok(captured[0] instanceof Error)
  } finally {
    fs.rmSync(views, { recursive: true, force: true })
  }
})

test('expressEngine resolves templates from any views root in order (multi-root)', async () => {
  const views1 = path.join(__dirname, `.express-mr1-${process.pid}`)
  const views2 = path.join(__dirname, `.express-mr2-${process.pid}`)
  fs.mkdirSync(views1, { recursive: true })
  fs.mkdirSync(views2, { recursive: true })
  fs.writeFileSync(path.join(views2, 'only-second.njk'), 'ONLY2', 'utf8')
  try {
    const app = express()
    app.set('views', [views1, views2])
    app.set('view engine', 'njk')
    expressEngine(app, { ext: 'njk' })
    app.get('/', (req, res) => {
      res.render('only-second', {})
    })
    const res = await request(app).get('/')
    assert.equal(res.status, 200)
    assert.equal(res.text, 'ONLY2')
  } finally {
    fs.rmSync(views1, { recursive: true, force: true })
    fs.rmSync(views2, { recursive: true, force: true })
  }
})

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

test('view cache off without invalidateOnViewCacheOff keeps cached template AST', async () => {
  const views = path.join(__dirname, `.express-inval-off-${process.pid}`)
  fs.mkdirSync(views, { recursive: true })
  const fp = path.join(views, 'live.njk')
  fs.writeFileSync(fp, 'v1', 'utf8')
  try {
    const app = express()
    app.set('views', views)
    app.set('view engine', 'njk')
    app.set('view cache', false)
    expressEngine(app, { ext: 'njk', invalidateOnViewCacheOff: false })
    app.get('/', (req, res) => {
      res.render('live', {})
    })
    const r1 = await request(app).get('/')
    assert.equal(r1.text, 'v1')
    fs.writeFileSync(fp, 'v2', 'utf8')
    const r2 = await request(app).get('/')
    assert.equal(r2.text, 'v1')
  } finally {
    fs.rmSync(views, { recursive: true, force: true })
  }
})
