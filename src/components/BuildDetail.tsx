import DownloadIcon from '@mui/icons-material/Download'
import MoreHorizIcon from '@mui/icons-material/MoreHoriz'
import Alert from '@mui/material/Alert'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import IconButton from '@mui/material/IconButton'
import Menu from '@mui/material/Menu'
import MenuItem from '@mui/material/MenuItem'
import Typography from '@mui/material/Typography'
import { useState } from 'react'
import type { Build, BuildChanges } from '../builds.js'
import { changesBetween, KIND_LABEL } from '../builds.js'
import { formatAbsolute, formatRelative } from '../format.js'

export type BusyAction = 'downloading' | 'activating' | 'removing'

export const BUSY_LABEL: Record<BusyAction, string> = {
  downloading: 'Downloading…',
  activating: 'Switching…',
  removing: 'Removing…',
}

interface BuildDetailProps {
  build: Build
  builds: Build[]
  busy?: BusyAction
  locked: boolean
  error?: string
  waitingForReload: boolean
  followingLabel?: string
  onUse: (tag: string) => void
  onDownload: (tag: string) => void
  onRemove: (tag: string) => void
  onCopyTag: (tag: string) => void
}

const DEFAULT_HEADING = "What's in it"

const CHANGES_HEADING: Record<BuildChanges['direction'], string> = {
  newer: 'Changes since your build',
  older: "Changes you'd lose",
}

function statusOf(build: Build, waitingForReload: boolean): string {
  if (build.active) {
    return waitingForReload
      ? 'in the folder, waiting for a Chrome reload'
      : 'active'
  }
  return build.installed ? 'downloaded' : 'not downloaded'
}

function actionLabel(build: Build, busy?: BusyAction): string {
  if (busy) {
    return BUSY_LABEL[busy]
  }
  return build.installed ? 'Use' : 'Get and use'
}

export function BuildDetail({
  build,
  builds,
  busy,
  locked,
  error,
  waitingForReload,
  followingLabel,
  onUse,
  onDownload,
  onRemove,
  onCopyTag,
}: BuildDetailProps) {
  const [menuAnchor, setMenuAnchor] = useState<HTMLElement | null>(null)
  const changes = build.active
    ? undefined
    : changesBetween(
        builds,
        builds.find((b) => b.active),
        build
      )
  const notes = changes?.notes ?? build.notes
  const notesHeading = changes
    ? CHANGES_HEADING[changes.direction]
    : DEFAULT_HEADING
  const kindPrefix =
    build.kind === 'nightly' ? '' : `${KIND_LABEL[build.kind]} · `

  function closeMenu() {
    setMenuAnchor(null)
  }

  return (
    <Box sx={{ maxWidth: 620 }}>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
        <Typography variant="h1" sx={{ flex: 1, minWidth: 0 }}>
          {build.title}
        </Typography>
        <IconButton
          aria-label="More actions"
          disabled={locked}
          onClick={(e) => {
            setMenuAnchor(e.currentTarget)
          }}
        >
          <MoreHorizIcon />
        </IconButton>
      </Box>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
        {kindPrefix}
        {build.version}
        {build.publishedAt ? (
          <span title={formatAbsolute(build.publishedAt)}>
            {' '}
            · built {formatRelative(build.publishedAt)}
          </span>
        ) : null}
        {' · '}
        {statusOf(build, waitingForReload)}
      </Typography>

      {build.active ? null : (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mt: 2.5 }}>
          <Button
            variant="contained"
            startIcon={build.installed ? undefined : <DownloadIcon />}
            loading={busy === 'downloading' || busy === 'activating'}
            loadingPosition="start"
            disabled={locked && busy === undefined}
            onClick={() => {
              onUse(build.tag)
            }}
          >
            {actionLabel(build, busy)}
          </Button>
        </Box>
      )}
      <Menu
        open={menuAnchor !== null}
        anchorEl={menuAnchor}
        onClose={closeMenu}
      >
        {build.installed ? null : (
          <MenuItem
            onClick={() => {
              closeMenu()
              onDownload(build.tag)
            }}
          >
            Download only
          </MenuItem>
        )}
        {build.installed && !build.active ? (
          <MenuItem
            onClick={() => {
              closeMenu()
              onRemove(build.tag)
            }}
            sx={{ color: 'error.main' }}
          >
            Remove downloaded copy
          </MenuItem>
        ) : null}
        <MenuItem
          onClick={() => {
            closeMenu()
            onCopyTag(build.tag)
          }}
        >
          Copy release tag
        </MenuItem>
      </Menu>
      {busy === 'removing' ? (
        <Typography variant="body2" color="text.secondary" sx={{ mt: 2 }}>
          {BUSY_LABEL.removing}
        </Typography>
      ) : null}
      {followingLabel && !build.active ? (
        <Typography
          variant="caption"
          color="text.secondary"
          component="p"
          sx={{ mt: 1 }}
        >
          Using a build yourself turns off following {followingLabel}.
        </Typography>
      ) : null}

      {error ? (
        <Alert severity="error" sx={{ mt: 2 }}>
          {error}
        </Alert>
      ) : null}

      {notes.length > 0 ? (
        <Box sx={{ mt: 4 }}>
          <Typography variant="h5">{notesHeading}</Typography>
          <Box component="ul" sx={{ mt: 1.5, pl: 2.5, '& li': { mb: 1 } }}>
            {notes.map((note) => (
              <li key={note}>
                <Typography variant="body1">{note}</Typography>
              </li>
            ))}
          </Box>
        </Box>
      ) : null}
    </Box>
  )
}
