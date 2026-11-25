import SyncIcon from '@mui/icons-material/Sync'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import MenuItem from '@mui/material/MenuItem'
import TextField from '@mui/material/TextField'
import Typography from '@mui/material/Typography'
import { formatAbsolute, formatRelative } from '../format.js'
import type { Train, TrainCheck, TrainKind } from '../types.js'

type TrainChoice = 'off' | TrainKind

const TRAIN_LABEL: Record<TrainKind, string> = {
  stable: 'Stable',
  nightly: 'Nightly',
}

const KEEP_CHOICES = ['all', '1', '2', '3', '5'] as const

interface AutoUpdateProps {
  train?: Train
  lastCheck?: TrainCheck
  checking: boolean
  titleOf: (tag: string) => string
  onChange: (train: Train | null) => void
  onCheck: () => void
}

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
  return `Last check failed: ${check.message ?? 'unknown error'}`
}

export function AutoUpdate({
  train,
  lastCheck,
  checking,
  titleOf,
  onChange,
  onCheck,
}: AutoUpdateProps) {
  const choice: TrainChoice = train?.kind ?? 'off'
  const keep = train?.keepLast === undefined ? 'all' : String(train.keepLast)

  function pickTrain(value: TrainChoice) {
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
    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, mt: 1.5 }}>
      <TextField
        select
        size="small"
        label="Auto update"
        value={choice}
        onChange={(e) => {
          pickTrain(e.target.value as TrainChoice)
        }}
        sx={{ width: 150 }}
      >
        <MenuItem value="off">Off</MenuItem>
        <MenuItem value="stable">Stable</MenuItem>
        <MenuItem value="nightly">Nightly</MenuItem>
      </TextField>
      {train ? (
        <>
          <TextField
            select
            size="small"
            label="Keep"
            value={keep}
            onChange={(e) => {
              pickKeep(e.target.value)
            }}
            sx={{ width: 150 }}
          >
            {KEEP_CHOICES.map((k) => (
              <MenuItem key={k} value={k}>
                {k === 'all' ? 'All builds' : `Newest ${k}`}
              </MenuItem>
            ))}
          </TextField>
          <Typography
            variant="body2"
            color={lastCheck?.status === 'failed' ? 'error' : 'text.secondary'}
            noWrap
            title={lastCheck ? formatAbsolute(lastCheck.checkedAt) : undefined}
            sx={{ flex: 1, minWidth: 0 }}
          >
            {checking ? 'Checking…' : describeCheck(train, lastCheck, titleOf)}
          </Typography>
          <Button
            size="small"
            startIcon={<SyncIcon />}
            loading={checking}
            onClick={onCheck}
          >
            Check now
          </Button>
        </>
      ) : (
        <Typography variant="body2" color="text.secondary">
          Pick Stable or Nightly to keep the active build on the newest one.
          Checks hourly while the app is open.
        </Typography>
      )}
    </Box>
  )
}
