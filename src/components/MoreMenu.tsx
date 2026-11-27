import MoreHorizIcon from '@mui/icons-material/MoreHoriz'
import Badge from '@mui/material/Badge'
import Box from '@mui/material/Box'
import Divider from '@mui/material/Divider'
import IconButton from '@mui/material/IconButton'
import Link from '@mui/material/Link'
import ListItemText from '@mui/material/ListItemText'
import Menu from '@mui/material/Menu'
import MenuItem from '@mui/material/MenuItem'
import Switch from '@mui/material/Switch'
import Typography from '@mui/material/Typography'
import { useState } from 'react'
import type { NotificationSettings } from '../types.js'
import type { AppUpdateStatus } from '../useAppUpdate.js'
import { canSelfUpdate } from '../useAppUpdate.js'

const CHECK_LABEL: Record<AppUpdateStatus, string> = {
  idle: 'Check for updates',
  checking: 'Checking…',
  upToDate: 'Up to date',
  failed: 'Check again',
}

const NOTIFICATION_LABEL: Record<keyof NotificationSettings, string> = {
  newBuilds: 'Notify me about new extension builds',
  appUpdates: 'Notify me about app updates',
}

interface ToggleItemProps {
  label: string
  checked: boolean
  onToggle: () => void
}

function ToggleItem({ label, checked, onToggle }: ToggleItemProps) {
  return (
    <MenuItem role="menuitemcheckbox" aria-checked={checked} onClick={onToggle}>
      <ListItemText>{label}</ListItemText>
      <Switch
        size="small"
        edge="end"
        checked={checked}
        tabIndex={-1}
        slotProps={{ input: { 'aria-hidden': true } }}
        sx={{ ml: 2, pointerEvents: 'none' }}
      />
    </MenuItem>
  )
}

interface MoreMenuProps {
  appVersion?: string
  appUpdateStatus: AppUpdateStatus
  appUpdateVersion?: string
  appUpdateBusy: boolean
  hasFolder: boolean
  notificationSettings?: NotificationSettings
  onToggleNotification: (key: keyof NotificationSettings) => void
  openAtLogin?: boolean
  onToggleOpenAtLogin: () => void
  onInstallAppUpdate: () => void
  onCheckAppUpdate: () => void
  onCopyFolder: () => void
  onShowFolder: () => void
  onShowSetup: () => void
  onReloadReleases: () => void
}

export function MoreMenu({
  appVersion,
  appUpdateStatus,
  appUpdateVersion,
  appUpdateBusy,
  hasFolder,
  notificationSettings,
  onToggleNotification,
  openAtLogin,
  onToggleOpenAtLogin,
  onInstallAppUpdate,
  onCheckAppUpdate,
  onCopyFolder,
  onShowFolder,
  onShowSetup,
  onReloadReleases,
}: MoreMenuProps) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null)

  function close() {
    setAnchor(null)
  }

  function run(action: () => void) {
    return () => {
      close()
      action()
    }
  }

  return (
    <>
      <IconButton
        aria-label={appUpdateVersion ? 'More, app update available' : 'More'}
        onClick={(e) => {
          setAnchor(e.currentTarget)
        }}
      >
        <Badge color="primary" variant="dot" invisible={!appUpdateVersion}>
          <MoreHorizIcon />
        </Badge>
      </IconButton>
      <Menu
        open={anchor !== null}
        anchorEl={anchor}
        onClose={close}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        transformOrigin={{ vertical: 'top', horizontal: 'right' }}
        slotProps={{ paper: { sx: { mt: 1, minWidth: 280 } } }}
      >
        {appUpdateVersion ? (
          <MenuItem
            disabled={appUpdateBusy}
            onClick={run(onInstallAppUpdate)}
            sx={{ color: 'primary.main', fontWeight: 600 }}
          >
            {appUpdateBusy
              ? 'Updating…'
              : `Restart to update to ${appUpdateVersion}`}
          </MenuItem>
        ) : null}
        {appUpdateVersion ? <Divider /> : null}
        <MenuItem disabled={!hasFolder} onClick={run(onCopyFolder)}>
          Copy folder path
        </MenuItem>
        <MenuItem disabled={!hasFolder} onClick={run(onShowFolder)}>
          Show folder
        </MenuItem>
        <MenuItem onClick={run(onShowSetup)}>Chrome setup steps</MenuItem>
        <Divider />
        {openAtLogin === undefined ? null : (
          <ToggleItem
            label="Open at login"
            checked={openAtLogin}
            onToggle={onToggleOpenAtLogin}
          />
        )}
        {notificationSettings
          ? (['newBuilds', 'appUpdates'] as const).map((key) => (
              <ToggleItem
                key={key}
                label={NOTIFICATION_LABEL[key]}
                checked={notificationSettings[key]}
                onToggle={() => {
                  onToggleNotification(key)
                }}
              />
            ))
          : null}
        <Divider />
        <MenuItem onClick={run(onReloadReleases)}>
          <ListItemText>Reload release list</ListItemText>
          <Typography variant="caption" color="text.secondary" sx={{ ml: 3 }}>
            ⌘R
          </Typography>
        </MenuItem>
        <Divider />
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: 2,
            px: 2,
            py: 0.75,
          }}
        >
          <Typography variant="caption" color="text.secondary">
            Danmaku Builds {appVersion}
          </Typography>
          {canSelfUpdate && !appUpdateVersion ? (
            <Link
              component="button"
              variant="caption"
              underline="hover"
              disabled={appUpdateStatus === 'checking'}
              onClick={onCheckAppUpdate}
            >
              {CHECK_LABEL[appUpdateStatus]}
            </Link>
          ) : null}
        </Box>
      </Menu>
    </>
  )
}
