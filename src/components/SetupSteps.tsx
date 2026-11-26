import CheckIcon from '@mui/icons-material/Check'
import DownloadIcon from '@mui/icons-material/Download'
import OpenInNewIcon from '@mui/icons-material/OpenInNew'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import Typography from '@mui/material/Typography'
import type { ReactNode } from 'react'
import { useState } from 'react'
import type { Build } from '../builds.js'
import type { BusyAction } from './BuildDetail.js'
import { BUSY_LABEL } from './BuildDetail.js'

interface SetupStepsProps {
  candidate?: Build
  active?: Build
  folder: string
  busy?: BusyAction
  locked: boolean
  onUse: (tag: string) => void
  onOpenExtensions: () => void
  onCopyFolder: () => Promise<void>
  onClose?: () => void
}

export function SetupSteps({
  candidate,
  active,
  folder,
  busy,
  locked,
  onUse,
  onOpenExtensions,
  onCopyFolder,
  onClose,
}: SetupStepsProps) {
  const [copied, setCopied] = useState(false)

  function copy() {
    void onCopyFolder().then(() => {
      setCopied(true)
      setTimeout(() => {
        setCopied(false)
      }, 1500)
    })
  }

  let buildStep: ReactNode
  if (active) {
    buildStep = (
      <Typography variant="body2" color="text.secondary">
        {active.title} is in the folder.
      </Typography>
    )
  } else if (candidate) {
    buildStep = (
      <>
        <Typography variant="body2" color="text.secondary">
          {candidate.title} is suggested. You can pick any build on the left.
        </Typography>
        <Button
          variant="contained"
          startIcon={<DownloadIcon />}
          loading={busy !== undefined}
          loadingPosition="start"
          disabled={locked && busy === undefined}
          onClick={() => {
            onUse(candidate.tag)
          }}
          sx={{ mt: 1.5 }}
        >
          {busy ? BUSY_LABEL[busy] : `Get and use ${candidate.title}`}
        </Button>
      </>
    )
  } else {
    buildStep = (
      <Typography variant="body2" color="text.secondary">
        Loading builds…
      </Typography>
    )
  }

  return (
    <Box sx={{ maxWidth: 600 }}>
      <Typography variant="h1">Set up Chrome</Typography>
      <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
        Three steps, once. After that, switching builds is one click and a
        reload.
      </Typography>
      <Box sx={{ mt: 3 }}>
        <Step number={1} done={active !== undefined} title="Get a build">
          {buildStep}
        </Step>
        <Step number={2} title="Turn on Developer mode">
          <Typography variant="body2" color="text.secondary">
            In Chrome extensions, top right.
          </Typography>
          <Button
            variant="outlined"
            startIcon={<OpenInNewIcon />}
            onClick={onOpenExtensions}
            sx={{ mt: 1.5 }}
          >
            Open Chrome extensions
          </Button>
        </Step>
        <Step number={3} title="Load unpacked and pick this folder">
          <Box
            sx={{
              mt: 1,
              display: 'flex',
              alignItems: 'center',
              gap: 1,
              pl: 1.5,
              pr: 0.5,
              py: 0.5,
              border: 1,
              borderColor: 'divider',
              borderRadius: 1,
              bgcolor: 'background.paper',
            }}
          >
            <Typography
              variant="body2"
              noWrap
              sx={{ flex: 1, fontFamily: 'monospace' }}
            >
              {folder}
            </Typography>
            <Button size="small" onClick={copy}>
              {copied ? 'Copied' : 'Copy'}
            </Button>
          </Box>
        </Step>
      </Box>
      {onClose ? (
        <Button onClick={onClose} sx={{ mt: 3 }}>
          Back to builds
        </Button>
      ) : null}
    </Box>
  )
}

interface StepProps {
  number: number
  title: string
  done?: boolean
  children: ReactNode
}

function Step({ number, title, done = false, children }: StepProps) {
  return (
    <Box
      sx={{
        display: 'flex',
        gap: 2,
        py: 2,
        borderBottom: 1,
        borderColor: 'divider',
      }}
    >
      <Box
        sx={{
          width: 24,
          height: 24,
          flexShrink: 0,
          borderRadius: '50%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: 12,
          border: 1,
          borderColor: done ? 'success.main' : 'divider',
          bgcolor: done ? 'success.main' : 'transparent',
          color: done ? 'success.contrastText' : 'text.secondary',
        }}
      >
        {done ? <CheckIcon sx={{ fontSize: 14 }} /> : number}
      </Box>
      <Box sx={{ flex: 1, minWidth: 0 }}>
        <Typography sx={{ fontWeight: 650, mb: 0.25 }}>{title}</Typography>
        {children}
      </Box>
    </Box>
  )
}
