import { getVersion } from '@tauri-apps/api/app'
import { useCallback, useEffect, useState } from 'react'
import type { Update } from './updater.js'
import { checkForUpdate, installUpdate, relaunchApp } from './updater.js'

const CHECK_INTERVAL_MS = 60 * 60 * 1000

export type AppUpdateStatus = 'idle' | 'checking' | 'upToDate' | 'failed'

// Only macOS and Linux get signed update bundles, so the Windows build has no
// entry in latest.json and a check there always fails.
export const canSelfUpdate = !navigator.userAgent.includes('Windows')

export function useAppUpdate(onError: (error: unknown) => void) {
  const [version, setVersion] = useState<string>()
  const [pending, setPending] = useState<Update | null>(null)
  const [status, setStatus] = useState<AppUpdateStatus>('idle')
  const [installing, setInstalling] = useState(false)

  const check = useCallback(
    async (manual: boolean) => {
      if (!canSelfUpdate) {
        return
      }
      setStatus('checking')
      try {
        const update = await checkForUpdate()
        setPending(update)
        setStatus(update ? 'idle' : 'upToDate')
      } catch (error) {
        setStatus('failed')
        if (manual) {
          onError(error)
        }
      }
    },
    [onError]
  )

  useEffect(() => {
    getVersion().then(setVersion).catch(onError)
  }, [onError])

  useEffect(() => {
    void check(false)
    const timer = setInterval(() => {
      void check(false)
    }, CHECK_INTERVAL_MS)
    return () => {
      clearInterval(timer)
    }
  }, [check])

  async function install() {
    if (!pending) {
      return
    }
    setInstalling(true)
    try {
      await installUpdate(pending)
      await relaunchApp()
    } catch (error) {
      onError(error)
      setInstalling(false)
    }
  }

  return {
    version,
    status,
    pendingVersion: pending?.version,
    installing,
    check: () => check(true),
    install,
  }
}
