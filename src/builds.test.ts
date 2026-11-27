import { describe, expect, it } from 'vitest'
import type { Build, BuildKind } from './builds.js'
import { changesBetween } from './builds.js'

function build(
  tag: string,
  kind: BuildKind,
  publishedAt: string | undefined,
  notes: string[],
  active = false
): Build {
  return {
    tag,
    version: '1.0.0',
    kind,
    title: tag,
    publishedAt,
    notes,
    installed: false,
    active,
  }
}

const stable = build('v1.2.0', 'stable', '2026-03-05T00:00:00Z', ['C', 'B'])
const n3 = build('nightly-3', 'nightly', '2026-03-04T00:00:00Z', ['C', 'B'])
const n2 = build('nightly-2', 'nightly', '2026-03-03T00:00:00Z', ['B', 'A'])
const n1 = build('nightly-1', 'nightly', '2026-03-02T00:00:00Z', ['A', 'Z'])
const base = build('nightly-0', 'nightly', '2026-03-01T00:00:00Z', ['Z'])
const pr = build('preview-pr-9', 'pr', '2026-03-03T12:00:00Z', ['PR only'])

function all(...extra: Build[]): Build[] {
  return [stable, n3, n2, n1, base, ...extra]
}

describe('changesBetween', () => {
  it('lists changes since the active build when the shown build is newer', () => {
    const active = { ...n1, active: true }
    const result = changesBetween(all(), active, n3)
    expect(result?.direction).toBe('newer')
    expect(result?.notes).toEqual(['C', 'B', 'A'])
  })

  it('lists what would be lost when the shown build is older', () => {
    const active = { ...n3, active: true }
    const result = changesBetween(all(), active, n1)
    expect(result?.direction).toBe('older')
    expect(result?.notes).toEqual(['C', 'B', 'A'])
  })

  it('removes duplicate lines across overlapping nightlies and a stable', () => {
    const active = { ...base, active: true }
    const result = changesBetween(all(), active, stable)
    expect(result?.notes).toEqual(['C', 'B', 'A', 'Z'])
  })

  it('excludes preview builds that sit between the two', () => {
    const active = { ...n1, active: true }
    const result = changesBetween(all(pr), active, n3)
    expect(result?.notes).not.toContain('PR only')
  })

  it('returns undefined when the shown build is a PR build', () => {
    const active = { ...n1, active: true }
    expect(changesBetween(all(pr), active, pr)).toBeUndefined()
  })

  it('returns undefined when the active build is a PR build', () => {
    const active = { ...pr, active: true }
    expect(changesBetween(all(pr), active, n3)).toBeUndefined()
  })

  it('returns undefined when there is no active build', () => {
    expect(changesBetween(all(), undefined, n3)).toBeUndefined()
  })

  it('returns undefined when a build has no publish date', () => {
    const undated = build('nightly-x', 'nightly', undefined, ['X'])
    const active = { ...n1, active: true }
    expect(changesBetween(all(undated), active, undated)).toBeUndefined()
  })

  it('returns undefined when the range has no notes', () => {
    const quiet = build('nightly-q', 'nightly', '2026-03-06T00:00:00Z', [])
    const active = { ...stable, active: true }
    expect(changesBetween(all(quiet), active, quiet)).toBeUndefined()
  })

  it('returns undefined for the active build itself', () => {
    const active = { ...n1, active: true }
    expect(changesBetween(all(), active, active)).toBeUndefined()
  })
})
