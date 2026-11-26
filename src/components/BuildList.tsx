import CheckIcon from '@mui/icons-material/Check'
import ClearIcon from '@mui/icons-material/Clear'
import SearchIcon from '@mui/icons-material/Search'
import Box from '@mui/material/Box'
import Button from '@mui/material/Button'
import CircularProgress from '@mui/material/CircularProgress'
import IconButton from '@mui/material/IconButton'
import InputAdornment from '@mui/material/InputAdornment'
import List from '@mui/material/List'
import ListItemButton from '@mui/material/ListItemButton'
import ListSubheader from '@mui/material/ListSubheader'
import MenuItem from '@mui/material/MenuItem'
import TextField from '@mui/material/TextField'
import Typography from '@mui/material/Typography'
import type { Ref } from 'react'
import type { Build, BuildKind } from '../builds.js'
import { matchesQuery } from '../builds.js'
import { formatDay } from '../format.js'

export type KindFilter = 'all' | Extract<BuildKind, 'stable' | 'nightly' | 'pr'>

const FILTER_LABEL: Record<KindFilter, string> = {
  all: 'All',
  stable: 'Stable',
  nightly: 'Nightly',
  pr: 'PRs',
}

const TAG_LABEL: Partial<Record<BuildKind, string>> = {
  stable: 'Stable',
  pr: 'PR',
  manual: 'Manual',
  branch: 'Branch',
  other: 'Preview',
}

interface BuildListProps {
  builds: Build[]
  selectedTag?: string
  busyTag?: string
  query: string
  filter: KindFilter
  searchRef: Ref<HTMLInputElement>
  hasMore: boolean
  loadingMore: boolean
  loaded: boolean
  onQueryChange: (query: string) => void
  onFilterChange: (filter: KindFilter) => void
  onSelect: (tag: string) => void
  onUse: (tag: string) => void
  onLoadMore: () => void
}

export function BuildList({
  builds,
  selectedTag,
  busyTag,
  query,
  filter,
  searchRef,
  hasMore,
  loadingMore,
  loaded,
  onQueryChange,
  onFilterChange,
  onSelect,
  onUse,
  onLoadMore,
}: BuildListProps) {
  const shown = builds.filter((b) => {
    if (filter !== 'all' && b.kind !== filter) {
      return false
    }
    return matchesQuery(b, query)
  })
  const groups: [string, Build[]][] = [
    ['On this computer', shown.filter((b) => b.installed)],
    ['On GitHub', shown.filter((b) => !b.installed)],
  ]

  let empty: string | undefined
  if (shown.length === 0) {
    empty = loaded ? 'No builds match.' : 'Loading builds…'
  }

  return (
    <Box
      sx={{
        width: 320,
        flexShrink: 0,
        display: 'flex',
        flexDirection: 'column',
        borderRight: 1,
        borderColor: 'divider',
      }}
    >
      <Box sx={{ display: 'flex', gap: 1, px: 1.5, pt: 1.5, pb: 1 }}>
        <TextField
          inputRef={searchRef}
          fullWidth
          size="small"
          value={query}
          placeholder="Search builds"
          onChange={(e) => {
            onQueryChange(e.target.value)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              onQueryChange('')
            }
          }}
          slotProps={{
            input: {
              startAdornment: (
                <InputAdornment position="start">
                  <SearchIcon fontSize="small" />
                </InputAdornment>
              ),
              endAdornment: query ? (
                <InputAdornment position="end">
                  <IconButton
                    size="small"
                    aria-label="Clear search"
                    onClick={() => {
                      onQueryChange('')
                    }}
                  >
                    <ClearIcon fontSize="small" />
                  </IconButton>
                </InputAdornment>
              ) : null,
            },
          }}
        />
        <TextField
          select
          size="small"
          value={filter}
          aria-label="Show"
          onChange={(e) => {
            onFilterChange(e.target.value as KindFilter)
          }}
          sx={{ width: 104, flexShrink: 0 }}
        >
          {(Object.keys(FILTER_LABEL) as KindFilter[]).map((k) => (
            <MenuItem key={k} value={k}>
              {FILTER_LABEL[k]}
            </MenuItem>
          ))}
        </TextField>
      </Box>
      <List
        disablePadding
        sx={{ flex: 1, overflow: 'auto', px: 1, pb: 2, scrollPaddingTop: 32 }}
      >
        {groups.map(([name, items]) => {
          if (items.length === 0) {
            return null
          }
          return (
            <li key={name}>
              <ul style={{ padding: 0 }}>
                <ListSubheader
                  sx={{
                    bgcolor: 'background.default',
                    color: 'text.secondary',
                    fontSize: 12,
                    fontWeight: 600,
                    lineHeight: '32px',
                    px: 1,
                  }}
                >
                  {name}
                </ListSubheader>
                {items.map((build) => (
                  <BuildRow
                    key={build.tag}
                    build={build}
                    selected={build.tag === selectedTag}
                    busy={build.tag === busyTag}
                    showKind={filter === 'all'}
                    onSelect={onSelect}
                    onUse={onUse}
                  />
                ))}
              </ul>
            </li>
          )
        })}
        {empty ? (
          <Typography
            variant="body2"
            color="text.secondary"
            sx={{ p: 2, textAlign: 'center' }}
          >
            {empty}
          </Typography>
        ) : null}
        {hasMore ? (
          <Box sx={{ display: 'flex', justifyContent: 'center', pt: 1 }}>
            <Button size="small" loading={loadingMore} onClick={onLoadMore}>
              Load older releases
            </Button>
          </Box>
        ) : null}
      </List>
    </Box>
  )
}

interface BuildRowProps {
  build: Build
  selected: boolean
  busy: boolean
  showKind: boolean
  onSelect: (tag: string) => void
  onUse: (tag: string) => void
}

function RowMark({ build, busy }: { build: Build; busy: boolean }) {
  if (busy) {
    return <CircularProgress size={12} />
  }
  if (build.active) {
    return (
      <Box
        aria-label="Active"
        sx={{
          width: 8,
          height: 8,
          borderRadius: '50%',
          bgcolor: 'success.main',
        }}
      />
    )
  }
  if (build.installed) {
    return (
      <CheckIcon
        aria-label="Downloaded"
        sx={{ fontSize: 14, color: 'text.secondary' }}
      />
    )
  }
  return null
}

function BuildRow({
  build,
  selected,
  busy,
  showKind,
  onSelect,
  onUse,
}: BuildRowProps) {
  const date = build.publishedAt ?? build.downloadedAt
  const kindTag = showKind ? TAG_LABEL[build.kind] : undefined

  return (
    <ListItemButton
      selected={selected}
      onClick={() => {
        onSelect(build.tag)
      }}
      onDoubleClick={() => {
        if (!build.active) {
          onUse(build.tag)
        }
      }}
      sx={{ borderRadius: 1, height: 38, gap: 1.25, px: 1 }}
    >
      <Box
        sx={{
          width: 14,
          display: 'flex',
          justifyContent: 'center',
          flexShrink: 0,
        }}
      >
        <RowMark build={build} busy={busy} />
      </Box>
      <Typography
        variant="body2"
        noWrap
        sx={{ flex: 1, minWidth: 0, fontWeight: 550, fontSize: 14 }}
      >
        {build.title}
      </Typography>
      {kindTag ? (
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            border: 1,
            borderColor: 'divider',
            borderRadius: 0.5,
            px: 0.625,
            lineHeight: '18px',
            flexShrink: 0,
          }}
        >
          {kindTag}
        </Typography>
      ) : null}
      <Typography
        variant="caption"
        color="text.secondary"
        sx={{ width: 48, textAlign: 'right', flexShrink: 0 }}
      >
        {date ? formatDay(date) : ''}
      </Typography>
    </ListItemButton>
  )
}
