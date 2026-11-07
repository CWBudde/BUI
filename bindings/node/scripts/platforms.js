// Stages the platform packages for publishing: for each platform in
// platform.js whose addon is in bindings/node (npm run build, or files
// downloaded from other CI runners), writes npm/<tag>/ with a package.json
// and that platform's addon and bui-runtime files. Prints the folders made.
//
// Stops when the optionalDependencies of package.json do not list each
// platform package at this package's own version.
'use strict'
const fs = require('node:fs')
const path = require('node:path')
const { TARGETS, packageName } = require('../platform.js')

const root = path.join(__dirname, '..')
const main = JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8'))

const want = Object.fromEntries(Object.keys(TARGETS).map((tag) => [packageName(tag), main.version]))
const have = main.optionalDependencies ?? {}
if (JSON.stringify(have) !== JSON.stringify(want)) {
  console.error('package.json: optionalDependencies must be exactly', want, 'got', have)
  process.exit(1)
}

for (const [tag, { os, cpu, libc }] of Object.entries(TARGETS)) {
  const addon = `blenderui.${tag}.node`
  if (!fs.existsSync(path.join(root, addon))) continue
  const dir = path.join(root, 'npm', tag)
  fs.rmSync(dir, { recursive: true, force: true })
  fs.mkdirSync(dir, { recursive: true })
  const files = fs.readdirSync(root).filter((f) => f === addon || (/^bui-runtime(-\w+)?\./.test(f) && f.includes(`.${tag}`)))
  for (const f of [...files, 'LICENSE']) fs.copyFileSync(path.join(root, f === 'LICENSE' ? '../../LICENSE' : f), path.join(dir, f))
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
}
