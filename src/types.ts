export type Channel = 'stable' | 'preview'

export type PreviewSubtype = 'nightly' | 'pr' | 'branch' | 'manual' | 'generic'

export interface ReleaseAsset {
  tag: string
  version: string
  channel: Channel
  previewSubtype?: PreviewSubtype
  publishedAt: string
  assetUrl: string
  notes: string[]
}

export interface CachedBuild {
  tag: string
  version: string
  channel: Channel
  downloadedAt: string
}

export type TrainKind = 'stable' | 'nightly'

export interface Train {
  kind: TrainKind
  keepLast?: number
}

export type TrainCheckStatus = 'upToDate' | 'updated' | 'notFound' | 'failed'

export interface TrainCheck {
  checkedAt: string
  status: TrainCheckStatus
  tag?: string
  message?: string
}

export interface PublicState {
  activeTag?: string
  activePath?: string
  dataDir: string
  builds: CachedBuild[]
  train?: Train
  pendingReload?: string
  lastTrainCheck?: TrainCheck
}
