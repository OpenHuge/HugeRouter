export const appThemeTokens = {
  color: {
    background: "#f6f8fb",
    border: "rgb(15 23 42 / 10%)",
    card: "#ffffff",
    foreground: "#10233b",
    muted: "rgb(15 23 42 / 5%)",
    mutedForeground: "#5c7394",
    primary: "#0f766e",
    primaryForeground: "#ffffff",
    ring: "#0f766e",
  },
  density: {
    compactRowHeight: 36,
    panelMaxWidth: 1280,
  },
  font: {
    body: "'IBM Plex Sans', 'Segoe UI', system-ui, sans-serif",
    heading: "'Space Grotesk', 'IBM Plex Sans', system-ui, sans-serif",
  },
  radius: {
    default: "0.625rem",
  },
} as const;

export const createAppTheme = () => appThemeTokens;

export type AppTheme = typeof appThemeTokens;
