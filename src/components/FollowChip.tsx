import ArrowDropDownIcon from '@mui/icons-material/ArrowDropDown'
import SyncIcon from '@mui/icons-material/Sync'
import SyncProblemIcon from '@mui/icons-material/SyncProblem'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import Chip from '@mui/material/Chip'
import CircularProgress from '@mui/material/CircularProgress'
import MenuItem from '@mui/material/MenuItem'
import Popover from '@mui/material/Popover'
import TextField from '@mui/material/TextField'
import ToggleButton from '@mui/material/ToggleButton'
import ToggleButtonGroup from '@mui/material/ToggleButtonGroup'
import Typography from '@mui/material/Typography'
import { useState } from 'react'
import { formatAbsolute, formatRelative } from '../format.js'
import type { Train, TrainCheck, TrainKind } from '../types.js'

type TrainChoice = 'off' | TrainKind

export const TRAIN_LABEL: Record<TrainKind, string> = {
  stable: 'Stable',
  nightly: 'Nightly',
}

const KEEP_CHOICES = ['all', '1', '2', '3', '5'] as const

function describeCheck(
  train: Train,
  check: TrainCheck | undefined,
  titleOf: (tag: string) => string
): string {
  if (!check) {
    return 'Not checked yet'
  }
  const when = formatRelative(check.checkedAt)
  if (check.status === 'updated' && check.tag) {
    return `Switched to ${titleOf(check.tag)} ${when}`
  }
  if (check.status === 'upToDate') {
    return `Up to date, checked ${when}`
  }
  if (check.status === 'notFound') {
    return `No ${TRAIN_LABEL[train.kind].toLowerCase()} builds found`
  }
  return `Check failed: ${check.message ?? 'unknown error'}`
}

function chipLabel(train: Train | undefined, failed: boolean): string {
  if (!train) {
    return 'Auto update off'
  }
  if (failed) {
    return 'Auto update failed'
  }
  return `Following ${TRAIN_LABEL[train.kind]}`
}

interface FollowChipProps {
  train?: Train
  lastCheck?: TrainCheck
  checking: boolean
  titleOf: (tag: string) => string
  onChange: (train: Train | null) => void
  onCheck: () => void
}

export function FollowChip({
  train,
  lastCheck,
  checking,
  titleOf,
  onChange,
  onCheck,
}: FollowChipProps) {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null)
  const failed = train !== undefined && lastCheck?.status === 'failed'
  const choice: TrainChoice = train?.kind ?? 'off'
  const keep = train?.keepLast === undefined ? 'all' : String(train.keepLast)

  let icon = <SyncIcon />
  if (checking) {
    icon = <CircularProgress size={14} sx={{ ml: '6px !important' }} />
  } else if (failed) {
    icon = <SyncProblemIcon />
  }

  function pickTrain(value: TrainChoice | null) {
    if (value === null || value === choice) {
      return
    }
    if (value === 'off') {
      onChange(null)
      return
    }
    onChange({ kind: value, keepLast: train?.keepLast })
  }

  function pickKeep(value: string) {
    if (!train) {
      return
    }
    const keepLast = value === 'all' ? undefined : Number(value)
    onChange({ kind: train.kind, keepLast })
  }

  return (
    <>
      <Chip
        icon={icon}
        label={
          <Box sx={{ display: 'flex', alignItems: 'center' }}>
            {chipLabel(train, failed)}
            <ArrowDropDownIcon fontSize="small" sx={{ mr: -0.75 }} />
          </Box>
        }
        variant="outlined"
        color={failed ? 'error' : 'default'}
        onClick={(e) => {
          setAnchor(e.currentTarget)
        }}
        sx={{
          height: 32,
          borderRadius: 4,
          px: 0.5,
          fontSize: 13,
          color: failed ? 'error.main' : 'text.primary',
        }}
      />
      <Popover
        open={anchor !== null}
        anchorEl={anchor}
        onClose={() => {
          setAnchor(null)
        }}
        anchorOrigin={{ vertical: 'bottom', horizontal: 'right' }}
        transformOrigin={{ vertical: 'top', horizontal: 'right' }}
        slotProps={{ paper: { sx: { mt: 1, p: 2, width: 320 } } }}
      >
        <Typography variant="caption" color="text.secondary">
          Keep the active build on the newest
        </Typography>
        <ToggleButtonGroup
          size="small"
          exclusive
          fullWidth
          value={choice}
          onChange={(_e, value: TrainChoice | null) => {
            pickTrain(value)
          }}
          sx={{ mt: 1 }}
        >
          <ToggleButton value="off">Off</ToggleButton>
          <ToggleButton value="stable">Stable</ToggleButton>
          <ToggleButton value="nightly">Nightly</ToggleButton>
        </ToggleButtonGroup>
        {train ? (
          <>
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                mt: 2,
              }}
            >
              <Typography variant="body2">Keep downloaded</Typography>
              <TextField
                select
                size="small"
                value={keep}
                onChange={(e) => {
                  pickKeep(e.target.value)
                }}
                sx={{ width: 140 }}
              >
                {KEEP_CHOICES.map((k) => (
                  <MenuItem key={k} value={k}>
                    {k === 'all' ? 'All builds' : `Newest ${k}`}
                  </MenuItem>
                ))}
              </TextField>
            </Box>
            <Box
              sx={{
                display: 'flex',
                alignItems: 'center',
                gap: 1,
                mt: 2,
                pt: 1.5,
                borderTop: 1,
                borderColor: 'divider',
              }}
            >
              <Typography
                variant="caption"
                color={failed ? 'error' : 'text.secondary'}
                title={
                  lastCheck ? formatAbsolute(lastCheck.checkedAt) : undefined
                }
                sx={{ flex: 1, minWidth: 0 }}
              >
                {checking
                  ? 'Checking…'
                  : describeCheck(train, lastCheck, titleOf)}
              </Typography>
              <Button size="small" loading={checking} onClick={onCheck}>
                Check now
              </Button>
            </Box>
          </>
        ) : (
          <Typography
            variant="caption"
            color="text.secondary"
            component="p"
            sx={{ mt: 1.5 }}
          >
            Pick Stable or Nightly to switch to each new build as it comes out.
            Checks hourly while the app is open.
          </Typography>
        )}
      </Popover>
    </>
  )
}
