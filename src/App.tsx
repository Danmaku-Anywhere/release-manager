import Alert from '@mui/material/Alert'
import Box from '@mui/material/Box'
import CircularProgress from '@mui/material/CircularProgress'
import Snackbar from '@mui/material/Snackbar'
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import * as api from './api.js'
import { toBuilds } from './builds.js'
import type { BusyAction } from './components/BuildDetail.js'
import { BuildDetail } from './components/BuildDetail.js'
import type { KindFilter } from './components/BuildList.js'
import { BuildList } from './components/BuildList.js'
import { FollowChip, TRAIN_LABEL } from './components/FollowChip.js'
import { MoreMenu } from './components/MoreMenu.js'
import { ReloadBanner } from './components/ReloadBanner.js'
import { SetupSteps } from './components/SetupSteps.js'
import { StatusStrip } from './components/StatusStrip.js'
import { openFolder } from './shell.js'
import type { PublicState, ReleaseAsset, Train } from './types.js'
import { useAppUpdate } from './useAppUpdate.js'
import { useNotificationSettings } from './useNotificationSettings.js'
import { useOpenAtLogin } from './useOpenAtLogin.js'

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
  const [stoppedTrain, setStoppedTrain] = useState<Train>()
  const [showSetup, setShowSetup] = useState(false)
  const [notice, setNotice] = useState<string>()
  const [releasesLoaded, setReleasesLoaded] = useState(false)
  const [checkingTrain, setCheckingTrain] = useState(false)
  const [query, setQuery] = useState('')
  const [filter, setFilter] = useState<KindFilter>('all')
  const [releasePage, setReleasePage] = useState(1)
  const [hasMoreReleases, setHasMoreReleases] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const reportError = useCallback((error: unknown) => {
    setGlobalError(errorMessage(error))
  }, [])
  const appUpdate = useAppUpdate(reportError)
  const notifications = useNotificationSettings(reportError)
  const openAtLogin = useOpenAtLogin(reportError)
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
    } finally {
      setReleasesLoaded(true)
    }
  }, [])

  const refreshAll = useCallback(async () => {
    await refreshState()
    await loadReleases()
  }, [refreshState, loadReleases])

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

  // The window is a webview: the browser's Ctrl/Cmd+F find is unavailable and
  // Ctrl/Cmd+R would reload the whole page, so both map to in-app actions.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (!(e.ctrlKey || e.metaKey)) {
        return
      }
      if (e.key === 'f') {
        e.preventDefault()
        searchRef.current?.focus()
        searchRef.current?.select()
      } else if (e.key === 'r') {
        e.preventDefault()
        void refreshAll()
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
    }
  }, [refreshAll])

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
  const newestStable = builds.find((b) => b.kind === 'stable')
  const selected =
    builds.find((b) => b.tag === selectedTag) ??
    active ??
    newestStable ??
    builds[0]
  const setupVisible = showSetup || (!active && selectedTag === undefined)

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
    const trainBefore = state?.train
    const switching = state?.activeTag !== tag
    if (await runAction(tag, steps)) {
      setStoppedTrain(switching ? trainBefore : undefined)
      setShowSetup(false)
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

  async function copyText(text: string, message: string) {
    try {
      await navigator.clipboard.writeText(text)
      setNotice(message)
    } catch (error) {
      showError(error)
    }
  }

  function selectBuild(tag: string) {
    setSelectedTag(tag)
    setShowSetup(false)
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
    setStoppedTrain(undefined)
    if (train) {
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

  const folder = state.activePath ?? `${state.dataDir}/active`
  const pending = state.pendingReload
  const followingLabel = state.train ? TRAIN_LABEL[state.train.kind] : undefined

  let detail = null
  if (setupVisible) {
    detail = (
      <SetupSteps
        candidate={selected}
        active={active}
        folder={folder}
        busy={busy?.tag === selected?.tag ? busy?.action : undefined}
        locked={busy !== undefined}
        onUse={(tag) => {
          void handleUse(tag)
        }}
        onOpenExtensions={handleOpenExtensions}
        onCopyFolder={() => copyText(folder, 'Folder path copied')}
        onClose={
          active
            ? () => {
                setShowSetup(false)
              }
            : undefined
        }
      />
    )
  } else if (selected) {
    detail = (
      <BuildDetail
        build={selected}
        builds={builds}
        busy={busy?.tag === selected.tag ? busy.action : undefined}
        locked={busy !== undefined}
        error={errors[selected.tag]}
        waitingForReload={pending === selected.tag}
        followingLabel={followingLabel}
        onUse={(tag) => {
          void handleUse(tag)
        }}
        onDownload={handleDownload}
        onRemove={handleRemove}
        onCopyTag={(tag) => {
          void copyText(tag, 'Release tag copied')
        }}
      />
    )
  }

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <StatusStrip
        active={active}
        showExtensionsButton={pending === undefined}
        onOpenExtensions={handleOpenExtensions}
        follow={
          <FollowChip
            train={state.train}
            lastCheck={state.lastTrainCheck}
            checking={checkingTrain}
            titleOf={titleOf}
            onChange={(train) => {
              void handleTrainChange(train)
            }}
            onCheck={() => {
              void handleCheckTrain()
            }}
          />
        }
        menu={
          <MoreMenu
            appVersion={appUpdate.version}
            appUpdateStatus={appUpdate.status}
            appUpdateVersion={appUpdate.pendingVersion}
            appUpdateBusy={appUpdate.installing}
            hasFolder={state.activePath !== undefined}
            onInstallAppUpdate={() => {
              void appUpdate.install()
            }}
            onCheckAppUpdate={() => {
              void appUpdate.check()
            }}
            onCopyFolder={() => {
              void copyText(folder, 'Folder path copied')
            }}
            onShowFolder={() => {
              openFolder(folder).catch(showError)
            }}
            onShowSetup={() => {
              setShowSetup(true)
            }}
            notificationSettings={notifications.settings}
            onToggleNotification={notifications.toggle}
            openAtLogin={openAtLogin.enabled}
            onToggleOpenAtLogin={openAtLogin.toggle}
            onReloadReleases={() => {
              void refreshAll()
            }}
          />
        }
      />
      {pending ? (
        <ReloadBanner
          title={titleOf(pending)}
          byAutoUpdate={
            state.train !== undefined && state.lastTrainCheck?.tag === pending
          }
          autoUpdateStopped={stoppedTrain !== undefined}
          onOpenExtensions={handleOpenExtensions}
          onUndoStop={() => {
            void handleTrainChange(stoppedTrain ?? null)
          }}
          onDone={() => {
            setStoppedTrain(undefined)
            api.dismissReload().then(setState).catch(showError)
          }}
        />
      ) : null}
      <Box sx={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <BuildList
          builds={builds}
          selectedTag={setupVisible ? undefined : selected?.tag}
          busyTag={busy?.tag}
          query={query}
          filter={filter}
          searchRef={searchRef}
          hasMore={hasMoreReleases}
          loadingMore={loadingMore}
          loaded={releasesLoaded}
          onQueryChange={setQuery}
          onFilterChange={setFilter}
          onSelect={selectBuild}
          onUse={(tag) => {
            selectBuild(tag)
            void handleUse(tag)
          }}
          onLoadMore={() => {
            void handleLoadMore()
          }}
        />
        <Box sx={{ flex: 1, overflow: 'auto', px: 4, py: 3.5 }}>{detail}</Box>
      </Box>
      <Snackbar
        open={notice !== undefined}
        autoHideDuration={2000}
        message={notice}
        onClose={() => {
          setNotice(undefined)
        }}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      />
      <Snackbar
        open={globalError !== undefined}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'left' }}
      >
        <Alert
          severity="error"
          variant="filled"
          onClose={() => {
            setGlobalError(undefined)
          }}
          sx={{ maxWidth: 560 }}
        >
          {globalError}
        </Alert>
      </Snackbar>
    </Box>
  )
}
