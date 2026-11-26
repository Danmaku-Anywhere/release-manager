const fs = require('node:fs')
const path = require('node:path')

const SEMVER = /^\d+\.\d+\.\d+$/

const version = process.argv[2]
if (!version || !SEMVER.test(version)) {
  console.error(`expected a version like 1.2.3, got "${version ?? ''}"`)
  process.exit(1)
}

const root = path.join(__dirname, '..')
for (const file of ['package.json', 'src-tauri/tauri.conf.json']) {
  const full = path.join(root, file)
  const json = JSON.parse(fs.readFileSync(full, 'utf8'))
  json.version = version
  fs.writeFileSync(full, `${JSON.stringify(json, null, 2)}\n`)
  console.log(`${file}: ${version}`)
}
