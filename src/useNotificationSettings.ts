import { useCallback, useEffect, useState } from 'react'
import { getNotificationSettings, setNotificationSettings } from './api.js'
import type { NotificationSettings } from './types.js'

export function useNotificationSettings(onError: (error: unknown) => void) {
  const [settings, setSettings] = useState<NotificationSettings>()

  useEffect(() => {
    getNotificationSettings().then(setSettings).catch(onError)
  }, [onError])

  const toggle = useCallback(
    (key: keyof NotificationSettings) => {
      if (!settings) {
        return
      }
      const next = { ...settings, [key]: !settings[key] }
      setSettings(next)
      setNotificationSettings(next).catch((error: unknown) => {
        setSettings(settings)
        onError(error)
      })
    },
    [settings, onError]
  )

  return { settings, toggle }
}
