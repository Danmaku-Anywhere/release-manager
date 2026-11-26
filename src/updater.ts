import { invoke } from '@tauri-apps/api/core'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { relaunch } from '@tauri-apps/plugin-process'
import type { Update } from '@tauri-apps/plugin-updater'
import { check } from '@tauri-apps/plugin-updater'

export type { Update }

export async function checkForUpdate(): Promise<Update | null> {
  return check()
}

export async function installUpdate(update: Update): Promise<void> {
  await update.downloadAndInstall()
}

export async function relaunchApp(): Promise<void> {
  await relaunch()
}

export async function setTrayUpdate(version: string | null): Promise<void> {
  await invoke('set_tray_update', { version })
}

export async function onTrayRequest(
  request: 'check' | 'install',
  handler: () => void
): Promise<UnlistenFn> {
  return listen(`tray-${request}-update`, handler)
}
