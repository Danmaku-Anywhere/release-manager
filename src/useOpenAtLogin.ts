import { useCallback, useEffect, useState } from 'react'
import { getOpenAtLogin, onOpenAtLoginChanged, setOpenAtLogin } from './api.js'

export function useOpenAtLogin(onError: (error: unknown) => void) {
  const [enabled, setEnabled] = useState<boolean>()

  useEffect(() => {
    getOpenAtLogin().then(setEnabled).catch(onError)
    const unlisten = onOpenAtLoginChanged(setEnabled)
    return () => {
      void unlisten.then((fn) => {
        fn()
      })
    }
  }, [onError])

  const toggle = useCallback(() => {
    if (enabled === undefined) {
      return
    }
    setEnabled(!enabled)
    setOpenAtLogin(!enabled).catch(onError)
  }, [enabled, onError])

  return { enabled, toggle }
}
