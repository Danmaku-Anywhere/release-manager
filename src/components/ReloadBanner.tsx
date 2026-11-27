import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import Link from '@mui/material/Link'
import { alpha } from '@mui/material/styles'
import Typography from '@mui/material/Typography'

interface ReloadBannerProps {
  title: string
  byAutoUpdate: boolean
  autoUpdateStopped: boolean
  onOpenExtensions: () => void
  onUndoStop: () => void
  onDone: () => void
}

export function ReloadBanner({
  title,
  byAutoUpdate,
  autoUpdateStopped,
  onOpenExtensions,
  onUndoStop,
  onDone,
}: ReloadBannerProps) {
  return (
    <Box
      role="status"
      sx={(theme) => ({
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
        px: 2,
        py: 1,
        bgcolor: alpha(theme.palette.warning.main, 0.14),
        borderBottom: 1,
        borderColor: alpha(theme.palette.warning.main, 0.25),
      })}
    >
      <Typography
        variant="body2"
        sx={(theme) => ({
          flex: 1,
          minWidth: 0,
          color: theme.palette.severityInk.warning,
        })}
      >
        {byAutoUpdate ? 'Auto update switched to ' : 'Switched to '}
        <Box component="b" sx={{ color: 'text.primary' }}>
          {title}
        </Box>
        . Danmaku Anywhere reloads itself within a minute. If it doesn't, reload
        it in Chrome.
        {autoUpdateStopped ? (
          <Box component="span" sx={{ color: 'text.secondary', ml: 1.5 }}>
            Auto update is now off.{' '}
            <Link
              component="button"
              variant="body2"
              color="inherit"
              onClick={onUndoStop}
              sx={{ verticalAlign: 'baseline' }}
            >
              Undo
            </Link>
          </Box>
        ) : null}
      </Typography>
      <Button
        variant="contained"
        color="warning"
        startIcon={<OpenInNewIcon />}
        onClick={onOpenExtensions}
        sx={{ flexShrink: 0 }}
      >
        Open Chrome extensions
      </Button>
      <Button color="warning" onClick={onDone} sx={{ flexShrink: 0 }}>
        I reloaded
      </Button>
    </Box>
  )
}
