import { createTheme } from '@mantine/core'

export const createAppTheme = () =>
  createTheme({
    primaryColor: 'teal',
    fontFamily: "'IBM Plex Sans', 'Segoe UI', sans-serif",
    headings: {
      fontFamily: "'Space Grotesk', 'IBM Plex Sans', sans-serif"
    },
    defaultRadius: 'md',
    colors: {
      slate: [
        '#f3f6fb',
        '#e7edf5',
        '#cfd9e6',
        '#b7c6d8',
        '#9fb3cb',
        '#879fbd',
        '#708bb0',
        '#5c7394',
        '#485b75',
        '#334156'
      ]
    },
    other: {
      density: {
        compactRowHeight: 36,
        panelMaxWidth: 1280
      }
    }
  })

export type AppTheme = ReturnType<typeof createAppTheme>

