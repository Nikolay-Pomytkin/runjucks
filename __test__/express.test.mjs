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
