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

/** The runtime `bui build` copies (as cli.js finds it): `name` from features.json. */
function runtimeFile (name = 'full') {
  const exe = process.platform === 'win32' ? '.exe' : ''
  const tag = `${process.platform}-${process.arch}`
  const small = path.join(__dirname, '..', `bui-runtime-${name}.${tag}${exe}`)
  if (name !== 'full' && fs.existsSync(small)) return small
  const shipped = path.join(__dirname, '..', `bui-runtime.${tag}${exe}`)
  if (fs.existsSync(shipped)) return shipped
  for (const profile of ['dist', 'release']) {
    const built = path.join(__dirname, '../../rust/target', profile, `bui-runtime${exe}`)
    if (fs.existsSync(built)) return built
  }
  return null
}

// bui-runtime (BUI + QuickJS, its image decoders included) measured 7,097,856
// bytes on Windows x64 (`npm run build`: the dist profile, fat LTO). The budget is
// that plus 5 %: growth past it should be a decision, not an accident.
const RUNTIME_BUDGET = { 'win32-x64': Math.round(7097856 * 1.05) }

function exec (file, args = []) {
  const r = spawnSync(file, args, { encoding: 'utf8', env: { ...process.env, BUI_HEADLESS: '1' }, timeout: 60000 })
  assert.strictEqual(r.status, 0, `${file} failed: ${r.stderr}`)
  return r.stdout
}

test('bui build: a QuickJS executable of a few MB runs the React app', { skip: !hasBun }, () => {
  const out = path.join(dir, `react-app${process.platform === 'win32' ? '.exe' : ''}`)
  const b = spawnSync(process.execPath, [cli, 'build', app, '-o', out, '--console'], { encoding: 'utf8' })
  assert.strictEqual(b.status, 0, b.stderr)
  // the executable is the runtime, the app's bundle and a 16-byte trailer
  // (cli.js build): bui build adds nothing else
  const exe = fs.readFileSync(out)
  const trailer = exe.subarray(exe.length - 16)
  assert.strictEqual(trailer.subarray(0, 8).toString(), 'BUIAPP01')
  const bundle = Number(trailer.readBigUInt64LE(8))
  // an app without images or video gets the smallest runtime (features.json)
  const chosen = /runtime "(\w+)"/.exec(b.stdout)[1]
  assert.strictEqual(chosen, 'base', b.stdout)
  const runtime = fs.statSync(runtimeFile(chosen)).size
  assert.strictEqual(exe.length, runtime + bundle + 16)
  const budget = RUNTIME_BUDGET[`${process.platform}-${process.arch}`]
  if (budget && runtimeFile(chosen).includes(`.${process.platform}-${process.arch}.exe`)) {
    assert.ok(runtime <= budget, `runtime ${runtime} bytes, budget ${budget}`)
  }
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
