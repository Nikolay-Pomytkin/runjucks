import assert from 'node:assert/strict'
import http from 'node:http'
import { once } from 'node:events'
import test from 'node:test'
import { Environment } from '../index.js'
import { fetchTemplateMap } from '../fetch-template-map.js'

test('fetch from local HTTP server then setTemplateMap + renderTemplate', async () => {
  const server = http.createServer((req, res) => {
    if (req.url === '/hi.njk') {
      res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8' })
      res.end('Hello {{ name }}')
      return
    }
    res.writeHead(404)
    res.end()
  })
  server.listen(0, '127.0.0.1')
  try {
    await once(server, 'listening')
    const { port } = server.address()
    const base = `http://127.0.0.1:${port}`
    const map = await fetchTemplateMap([{ name: 'hi.njk', url: `${base}/hi.njk` }])
    const env = new Environment()
    env.setTemplateMap(map)
    assert.equal(env.renderTemplate('hi.njk', { name: 'Ada' }), 'Hello Ada')
  } finally {
    server.close()
    await once(server, 'close')
  }
})

test('setLoaderCallback closure over fetch result', async () => {
  const server = http.createServer((req, res) => {
    if (req.url === '/x.njk') {
      res.writeHead(200)
      res.end('v={{ v }}')
      return
    }
    res.writeHead(404)
    res.end()
  })
  server.listen(0, '127.0.0.1')
  try {
    await once(server, 'listening')
    const { port } = server.address()
    const url = `http://127.0.0.1:${port}/x.njk`
    const res = await fetch(url)
    const src = await res.text()
    const map = { 'x.njk': src }
    const env = new Environment()
    env.setLoaderCallback((name) => map[name] ?? null)
    assert.equal(env.renderTemplate('x.njk', { v: 2 }), 'v=2')
  } finally {
    server.close()
    await once(server, 'close')
  }
})
