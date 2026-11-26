import Alert from '@mui/material/Alert'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import * as api from './api.js'
import { toBuilds } from './builds.js'
import type { BusyAction } from './components/BuildDetail.js'
import { BuildDetail } from './components/BuildDetail.js'
import type { KindFilter } from './components/BuildList.js'
import { BuildList } from './components/BuildList.js'
import { TopBar } from './components/TopBar.js'
import { openFolder } from './shell.js'
import type { PublicState, ReleaseAsset, Train } from './types.js'
import { useAppUpdate } from './useAppUpdate.js'

const PAGE_SIZE = 100

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

export function App() {
  const [state, setState] = useState<PublicState | null>(null)
  const [releases, setReleases] = useState<ReleaseAsset[]>([])
  const [globalError, setGlobalError] = useState<string | undefined>()
  const [errors, setErrors] = useState<Record<string, string>>({})
  const [busy, setBusy] = useState<{ tag: string; action: BusyAction }>()
  const [selectedTag, setSelectedTag] = useState<string>()
  const [reloadNeeded, setReloadNeeded] = useState(false)
  const [trainStopped, setTrainStopped] = useState(false)
  const [checkingTrain, setCheckingTrain] = useState(false)
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<KindFilter>('all')
  const [releasePage, setReleasePage] = useState(1)
  const [hasMoreReleases, setHasMoreReleases] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [refreshing, setRefreshing] = useState(false)
  const reportError = useCallback((error: unknown) => {
    setGlobalError(errorMessage(error))
  }, [])
  const appUpdate = useAppUpdate(reportError)
  const searchRef = useRef<HTMLInputElement>(null)

  const refreshState = useCallback(async () => {
    try {
      setState(await api.getState())
    } catch (error) {
      setGlobalError(errorMessage(error))
    }
  }, [])

  const loadReleases = useCallback(async () => {
    setGlobalError(undefined)
    try {
      const page1 = await api.getReleases(1)
      setReleases(page1)
      setReleasePage(1)
      setHasMoreReleases(page1.length === PAGE_SIZE)
    } catch (error) {
      setGlobalError(errorMessage(error))
    }
  }, [])

  useEffect(() => {
    void refreshState()
    void loadReleases()
  }, [refreshState, loadReleases])

  useEffect(() => {
    const unlisten = api.onStateChanged(setState)
    return () => {
      void unlisten.then((stop) => {
        stop()
      })
    }
  }, [])

  // The window is a webview, so the browser's Ctrl/Cmd+F find is unavailable.
  // Reuse the shortcut to focus the in-app search field instead.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'f' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault()
        searchRef.current?.focus()
        searchRef.current?.select()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [])

  const builds = useMemo(() => {
    if (!state) {
      return []
    }
    return toBuilds(releases, state)
  }, [releases, state])

  function titleOf(tag: string): string {
    return builds.find((b) => b.tag === tag)?.title ?? tag
  }

  const active = builds.find((b) => b.active)
  const selected =
    builds.find((b) => b.tag === selectedTag) ?? active ?? builds[0]

  async function runAction(
    tag: string,
    steps: [BusyAction, () => Promise<PublicState>][]
  ): Promise<boolean> {
    if (busy) {
      return false
    }
    setErrors((prev) => {
      const next = { ...prev }
      delete next[tag]
      return next
    })
    try {
      for (const [action, run] of steps) {
        setBusy({ tag, action })
        setState(await run())
      }
      return true
    } catch (error) {
      setErrors((prev) => ({ ...prev, [tag]: errorMessage(error) }))
      return false
    } finally {
      setBusy(undefined)
    }
  }

  async function handleUse(tag: string) {
    const installed = state?.builds.some((b) => b.tag === tag) ?? false
    const steps: [BusyAction, () => Promise<PublicState>][] = []
    if (!installed) {
      steps.push(['downloading', () => api.downloadBuild(tag)])
    }
    steps.push(['activating', () => api.setActive(tag)])
    const hadTrain = state?.train !== undefined
    if (await runAction(tag, steps)) {
      setReloadNeeded(true)
      setTrainStopped(hadTrain && state?.activeTag !== tag)
    }
  }

  function handleDownload(tag: string) {
    void runAction(tag, [['downloading', () => api.downloadBuild(tag)]])
  }

  function handleRemove(tag: string) {
    void runAction(tag, [['removing', () => api.removeBuild(tag)]])
  }

  function showError(error: unknown) {
    setGlobalError(errorMessage(error))
  }

  async function handleCheckTrain() {
    setCheckingTrain(true)
    try {
      setState(await api.checkTrain())
    } catch (_error) {
      await refreshState()
    } finally {
      setCheckingTrain(false)
    }
  }

  async function handleTrainChange(train: Train | null) {
    try {
      setState(await api.setTrain(train))
    } catch (error) {
      showError(error)
      return
    }
    if (train) {
      setTrainStopped(false)
      await handleCheckTrain()
    }
  }

  function handleOpenExtensions() {
    api.openExtensionsPage().catch(showError)
  }

  async function handleLoadMore() {
    const nextPage = releasePage + 1
    setLoadingMore(true)
    setGlobalError(undefined)
    try {
      const more = await api.getReleases(nextPage)
      setReleases((prev) => {
        const seen = new Set(prev.map((r) => r.tag))
        return [...prev, ...more.filter((r) => !seen.has(r.tag))]
      })
      setReleasePage(nextPage)
      setHasMoreReleases(more.length === PAGE_SIZE)
    } catch (error) {
      setGlobalError(errorMessage(error))
    } finally {
      setLoadingMore(false)
    }
  }

  async function handleRefresh() {
    setRefreshing(true)
    try {
      await refreshState()
      await loadReleases()
    } finally {
      setRefreshing(false)
    }
  }

  if (state === null) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', pt: 8 }}>
        {globalError ? (
          <Alert severity="error">{globalError}</Alert>
        ) : (
          <CircularProgress />
        )}
      </Box>
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <TopBar
        active={active}
        activePath={state.activePath}
        reloadNeeded={reloadNeeded}
        trainStopped={trainStopped}
        autoReloadTitle={
          state.pendingReload ? titleOf(state.pendingReload) : undefined
        }
        train={state.train}
        lastTrainCheck={state.lastTrainCheck}
        checkingTrain={checkingTrain}
        titleOf={titleOf}
        appVersion={appUpdate.version}
        appUpdateStatus={appUpdate.status}
        appUpdateVersion={appUpdate.pendingVersion}
        appUpdateBusy={appUpdate.installing}
        onOpenExtensions={handleOpenExtensions}
        onShowFolder={() => {
          if (state.activePath) {
            openFolder(state.activePath).catch(showError)
          }
        }}
        onError={showError}
        onDismissReload={() => {
          setReloadNeeded(false)
          setTrainStopped(false)
        }}
        onDismissAutoReload={() => {
          api.dismissReload().then(setState).catch(showError)
        }}
        onTrainChange={(train) => {
          void handleTrainChange(train)
        }}
        onCheckTrain={() => {
          void handleCheckTrain()
        }}
        onInstallAppUpdate={() => {
          void appUpdate.install()
        }}
        onCheckAppUpdate={() => {
          void appUpdate.check()
        }}
      />
      <Box sx={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <BuildList
          builds={builds}
          selectedTag={selected?.tag}
          busy={busy}
          locked={busy !== undefined}
          query={query}
          filter={filter}
          searchRef={searchRef}
          hasMore={hasMoreReleases}
          loadingMore={loadingMore}
          refreshing={refreshing}
          onQueryChange={setQuery}
          onFilterChange={setFilter}
          onSelect={setSelectedTag}
          onUse={(tag) => {
            void handleUse(tag)
          }}
          onLoadMore={() => {
            void handleLoadMore()
          }}
          onRefresh={() => {
            void handleRefresh()
          }}
        />
        <Box sx={{ flex: 1, overflow: 'auto', px: 4, py: 3 }}>
          {globalError ? (
            <Alert
              severity="error"
              sx={{ mb: 3 }}
              onClose={() => {
                setGlobalError(undefined)
              }}
            >
              {globalError}
            </Alert>
          ) : null}
          {selected ? (
            <BuildDetail
              build={selected}
              busy={busy?.tag === selected.tag ? busy.action : undefined}
              locked={busy !== undefined}
              error={errors[selected.tag]}
              onUse={(tag) => {
                void handleUse(tag)
              }}
              onDownload={handleDownload}
              onRemove={handleRemove}
            />
          ) : null}
        </Box>
      </Box>
    </Box>
  )
}
