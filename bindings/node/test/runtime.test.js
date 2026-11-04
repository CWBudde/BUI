'use strict'
// `bui build` / `bui run`: the same React app as a QuickJS executable, a Bun
// executable, and on the QuickJS runtime directly.
const test = require('node:test')
const assert = require('node:assert')
const fs = require('node:fs')
const os = require('node:os')
const path = require('node:path')
const { spawnSync } = require('node:child_process')

const cli = path.join(__dirname, '..', 'cli.js')
const app = path.join(__dirname, 'fixtures', 'react-app.jsx')
const hasBun = spawnSync('bun', ['--version'], { shell: true }).status === 0
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bui-test-'))
test.after(() => fs.rmSync(dir, { recursive: true, force: true }))

function check (stdout, runtime) {
  const line = stdout.trim().split('\n').pop()
  const r = JSON.parse(line)
  assert.strictEqual(r.runtime, runtime)
  assert.deepStrictEqual(r.before, [0, 0, 255], 'rendered')
  assert.deepStrictEqual(r.after, [0, 255, 0], 'a signal prop updated after the click')
  assert.strictEqual(r.once, 'count 1', 'a signal child updated')
  assert.strictEqual(r.rendersAfterOne, 1, 'no re-render for signals passed on')
  assert.strictEqual(r.many, true, 'a render that read the signal re-rendered')
  return r
}

function exec (file, args = []) {
  const r = spawnSync(file, args, { encoding: 'utf8', env: { ...process.env, BUI_HEADLESS: '1' }, timeout: 60000 })
  assert.strictEqual(r.status, 0, `${file} failed: ${r.stderr}`)
  return r.stdout
}

test('bui build: a QuickJS executable of a few MB runs the React app', { skip: !hasBun }, () => {
  const out = path.join(dir, `react-app${process.platform === 'win32' ? '.exe' : ''}`)
  const b = spawnSync(process.execPath, [cli, 'build', app, '-o', out, '--console'], { encoding: 'utf8' })
  assert.strictEqual(b.status, 0, b.stderr)
  const size = fs.statSync(out).size / 1048576
  assert.ok(size < 8, `small: ${size.toFixed(1)} MB`)
  check(exec(out), 'quickjs')
})

test('bui run: the same app on the QuickJS runtime, no executable', { skip: !hasBun }, () => {
  const r = spawnSync(process.execPath, [cli, 'run', app], { encoding: 'utf8', env: { ...process.env, BUI_HEADLESS: '1' } })
  assert.strictEqual(r.status, 0, r.stderr)
  check(r.stdout, 'quickjs')
})

test('bui build --bun: a Bun executable runs it too', { skip: !hasBun }, () => {
  const out = path.join(dir, `react-app-bun${process.platform === 'win32' ? '.exe' : ''}`)
  const b = spawnSync(process.execPath, [cli, 'build', app, '-o', out, '--bun', '--console'], { encoding: 'utf8' })
  assert.strictEqual(b.status, 0, b.stderr)
  check(exec(out), 'bun')
})
