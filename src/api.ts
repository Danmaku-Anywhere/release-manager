import { invoke } from '@tauri-apps/api/core'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import type {
  NotificationSettings,
  PublicState,
  ReleaseAsset,
  Train,
} from './types.js'

// invoke() rejects with the serialized RmError object {kind, message} rather than
// a JS Error, so String(err) would show "[object Object]" in the UI. Unwrap it here.
function unwrapError(err: unknown): never {
  if (typeof err === 'object' && err !== null && 'message' in err) {
    const msg = (err as { message: unknown }).message
    if (typeof msg === 'string') {
      throw new Error(msg)
    }
  }
  throw err
}

export async function getState(): Promise<PublicState> {
  return invoke<PublicState>('get_state').catch(unwrapError)
}

export async function getReleases(page = 1): Promise<ReleaseAsset[]> {
  return invoke<ReleaseAsset[]>('list_releases', { page }).catch(unwrapError)
}

export async function downloadBuild(tag: string): Promise<PublicState> {
  return invoke<PublicState>('download_build', { tag }).catch(unwrapError)
}

export async function setActive(tag: string): Promise<PublicState> {
  return invoke<PublicState>('set_active', { tag }).catch(unwrapError)
}

export async function removeBuild(tag: string): Promise<PublicState> {
  return invoke<PublicState>('remove_build', { tag }).catch(unwrapError)
}

export async function setTrain(train: Train | null): Promise<PublicState> {
  return invoke<PublicState>('set_train', { train }).catch(unwrapError)
}

export async function checkTrain(): Promise<PublicState> {
  return invoke<PublicState>('check_train').catch(unwrapError)
}

export async function dismissReload(): Promise<PublicState> {
  return invoke<PublicState>('dismiss_reload').catch(unwrapError)
}

export function onStateChanged(
  handler: (state: PublicState) => void
): Promise<UnlistenFn> {
  return listen<PublicState>('state-changed', (event) => {
    handler(event.payload)
  })
}

export async function openExtensionsPage(): Promise<void> {
  return invoke<void>('open_extensions_page').catch(unwrapError)
}

export async function getNotificationSettings(): Promise<NotificationSettings> {
  return invoke<NotificationSettings>('get_notification_settings').catch(
    unwrapError
  )
}

export async function setNotificationSettings(
  settings: NotificationSettings
): Promise<NotificationSettings> {
  return invoke<NotificationSettings>('set_notification_settings', {
    settings,
  }).catch(unwrapError)
}

export async function getOpenAtLogin(): Promise<boolean> {
  return invoke<boolean>('get_open_at_login')
}

export async function setOpenAtLogin(enabled: boolean): Promise<boolean> {
  return invoke<boolean>('set_open_at_login', { enabled })
}

export function onOpenAtLoginChanged(
  handler: (enabled: boolean) => void
): Promise<UnlistenFn> {
  return listen<boolean>('open-at-login-changed', (event) => {
    handler(event.payload)
  })
}
