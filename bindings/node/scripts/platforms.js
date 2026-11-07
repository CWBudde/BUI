// Stages the platform packages for publishing: for each platform in
// platform.js whose addon is in bindings/node (npm run build, or files
// downloaded from other CI runners), writes npm/<tag>/ with a package.json
// and that platform's addon and bui-runtime files. Prints the folders made.
//
// --main also writes the optionalDependencies of package.json (all platform
// packages, at this version), as napi-rs's prepublish does: the lockfile then
// never names packages that are not published yet. It stops if a platform
// is not staged. Release CI only: do not commit the change.
'use strict'
const fs = require('node:fs')
const path = require('node:path')
const { TARGETS, packageName } = require('../platform.js')

const root = path.join(__dirname, '..')
const mainPath = path.join(root, 'package.json')
const main = JSON.parse(fs.readFileSync(mainPath, 'utf8'))
const runtime = /^bui-runtime(-\w+)?\./

function stage (tag, { os, cpu, libc }) {
  const addon = `blenderui.${tag}.node`
  if (!fs.existsSync(path.join(root, addon))) return false
  const dir = path.join(root, 'npm', tag)
  fs.rmSync(dir, { recursive: true, force: true })
  fs.mkdirSync(dir, { recursive: true })
  const files = fs.readdirSync(root).filter((f) => f === addon || (runtime.test(f) && f.includes(`.${tag}`)))
  for (const f of files) fs.copyFileSync(path.join(root, f), path.join(dir, f))
  fs.copyFileSync(path.join(root, '../../LICENSE'), path.join(dir, 'LICENSE'))
  const pkg = {
    name: packageName(tag),
    version: main.version,
    description: `The ${tag} native addon and bui-runtime of ${main.name}.`,
    main: addon,
    files: [...files, 'LICENSE'],
    os: [os],
    cpu: [cpu],
    ...(libc && { libc: [libc] }),
    license: main.license,
    repository: main.repository,
    homepage: main.homepage,
  }
  fs.writeFileSync(path.join(dir, 'package.json'), `${JSON.stringify(pkg, null, 2)}\n`)
  console.log(dir)
  return true
}

const missing = Object.entries(TARGETS).filter(([tag, target]) => !stage(tag, target)).map(([tag]) => tag)
if (process.argv.includes('--main')) {
  if (missing.length) {
    console.error(`no addon for ${missing.join(', ')}: ${main.name} would list platform packages that are not published`)
    process.exit(1)
  }
  main.optionalDependencies = Object.fromEntries(Object.keys(TARGETS).map((tag) => [packageName(tag), main.version]))
  fs.writeFileSync(mainPath, `${JSON.stringify(main, null, 2)}\n`)
}
