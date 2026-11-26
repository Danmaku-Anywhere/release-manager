import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import Typography from '@mui/material/Typography'
import type { ReactNode } from 'react'
import type { Build } from '../builds.js'
import { formatDay } from '../format.js'

interface StatusStripProps {
  active?: Build
  showExtensionsButton: boolean
  follow: ReactNode
  menu: ReactNode
  onOpenExtensions: () => void
}

export function StatusStrip({
  active,
  showExtensionsButton,
  follow,
  menu,
  onOpenExtensions,
}: StatusStripProps) {
  const date = active?.publishedAt ?? active?.downloadedAt

  return (
    <Box
      sx={{
        height: 56,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
        px: 2,
        bgcolor: 'background.paper',
        borderBottom: 1,
        borderColor: 'divider',
      }}
    >
      <Box
        sx={{
          flex: 1,
          minWidth: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 1,
        }}
      >
        <Box
          sx={{
            width: 8,
            height: 8,
            borderRadius: '50%',
            flexShrink: 0,
            bgcolor: active ? 'success.main' : 'text.disabled',
          }}
        />
        {active ? (
          <Typography noWrap sx={{ minWidth: 0 }}>
            <Typography component="span" variant="body2" color="text.secondary">
              Active
            </Typography>{' '}
            <Typography component="span" sx={{ fontWeight: 650, mx: 0.5 }}>
              {active.title}
            </Typography>{' '}
            <Typography
              component="span"
              variant="caption"
              color="text.secondary"
            >
              {active.version}
              {date ? ` · ${formatDay(date)}` : ''}
            </Typography>
          </Typography>
        ) : (
          <Typography noWrap sx={{ fontWeight: 650 }}>
            No build active
          </Typography>
        )}
      </Box>
      {follow}
      {showExtensionsButton ? (
        <Button
          variant="outlined"
          startIcon={<OpenInNewIcon />}
          onClick={onOpenExtensions}
        >
          Chrome extensions
        </Button>
      ) : null}
      {menu}
    </Box>
  )
}
