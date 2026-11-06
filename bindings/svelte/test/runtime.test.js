// The same Svelte app on every runtime: Node.js and Bun (through register.js),
// and as `bui build` / `bui run` apps (QuickJS, and a Bun executable), with
// the Svelte plugin found through this package's `bui.bunPlugin`.
import test from 'node:test'
import assert from 'node:assert'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { fileURLToPath, pathToFileURL } from 'node:url'

const here = path.dirname(fileURLToPath(import.meta.url))
const cli = path.join(here, '..', '..', 'node', 'cli.js')
const register = path.join(here, '..', 'register.js')
const app = path.join(here, 'fixtures', 'app.js')
// Bun's executable (npm installs it behind a shell shim on Windows)
const bun = spawnSync('bun', ['-e', 'process.stdout.write(process.execPath)'], { encoding: 'utf8', shell: true }).stdout?.trim()
const hasBun = Boolean(bun)
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'bui-svelte-test-'))
test.after(() => fs.rmSync(dir, { recursive: true, force: true }))
const env = { ...process.env, BUI_HEADLESS: '1' }

function check (r, runtime) {
  assert.strictEqual(r.status, 0, r.stderr)
  const out = JSON.parse(r.stdout.trim().split('\n').pop())
  assert.strictEqual(out.runtime, runtime)
  assert.deepStrictEqual(out.before, [0, 0, 255], 'rendered')
  assert.deepStrictEqual(out.after, [0, 255, 0], 'state updated after the click')
  assert.strictEqual(out.once, 'count 1', 'the text followed the state')
  assert.strictEqual(out.many, true, 'a conditional block mounted')
  assert.strictEqual(out.renders, 1, 'the component ran once')
}

const run = (file, args = []) => spawnSync(file, args, { encoding: 'utf8', env, timeout: 60000 })
const exe = process.platform === 'win32' ? '.exe' : ''

test('node --import @blender-ui/svelte/register runs the app', () => {
  check(run(process.execPath, ['--import', pathToFileURL(register).href, app]), 'node')
})

test('bun --preload @blender-ui/svelte/register runs it', { skip: !hasBun }, () => {
  check(run(bun, ['--preload', register, app]), 'bun')
})

test('bui build: a QuickJS executable of a few MB runs the Svelte app', { skip: !hasBun }, () => {
  const out = path.join(dir, `svelte-app${exe}`)
  const b = spawnSync(process.execPath, [cli, 'build', app, '-o', out, '--console'], { encoding: 'utf8' })
  assert.strictEqual(b.status, 0, b.stderr)
  const size = fs.statSync(out).size / 1048576
  assert.ok(size < 8, `small: ${size.toFixed(1)} MB`)
  check(run(out), 'quickjs')
})

test('bui run: the same app on the QuickJS runtime, no executable', { skip: !hasBun }, () => {
  check(spawnSync(process.execPath, [cli, 'run', app], { encoding: 'utf8', env }), 'quickjs')
})

test('bui build --bun: a Bun executable runs it too', { skip: !hasBun }, () => {
  const out = path.join(dir, `svelte-app-bun${exe}`)
  const b = spawnSync(process.execPath, [cli, 'build', app, '-o', out, '--bun', '--console'], { encoding: 'utf8' })
  assert.strictEqual(b.status, 0, b.stderr)
  check(run(out), 'bun')
})
