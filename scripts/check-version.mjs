#!/usr/bin/env node
// Author: Kirin
/**
 * 版本一致性校验。
 *
 * 用法:
 *   node scripts/check-version.mjs            # 只比对项目内的三处版本
 *   node scripts/check-version.mjs v0.1.2     # 额外比对 git 标签
 *
 * 背景: v0.1.1 发布出来的安装包文件名是 Agent.Manager_0.1.0_...，
 * 因为当时 tauri.conf.json 没跟着升版本，CI 静默产出了版本对不上的产物。
 */
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const read = (relative) => readFileSync(join(root, relative), 'utf8')

const cargoVersion = read('src-tauri/Cargo.toml').match(/^version\s*=\s*"([^"]+)"/m)?.[1]

const sources = [
  ['package.json', JSON.parse(read('package.json')).version],
  ['src-tauri/tauri.conf.json', JSON.parse(read('src-tauri/tauri.conf.json')).version],
  ['src-tauri/Cargo.toml', cargoVersion],
]

const tag = (process.argv[2] ?? '').trim().replace(/^v/, '')
if (tag) sources.push([`git 标签 v${tag}`, tag])

const expected = sources[0][1]
let failed = !expected

for (const [label, version] of sources) {
  const ok = Boolean(version) && version === expected
  if (!ok) failed = true
  console.log(`${ok ? '✓' : '✗'} ${label.padEnd(26)} ${version ?? '(读不到版本)'}`)
}

if (failed) {
  console.error('\n版本不一致：以上几处必须完全相同，否则会产出文件名和版本对不上的安装包。')
  process.exit(1)
}

console.log(`\n版本一致：${expected}`)
