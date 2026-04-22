import { AppShell, Box, Group, NavLink, Stack, Text } from '@mantine/core'
import type { ReactNode } from 'react'

export type ShellNavItem = {
  href: string
  label: string
  description: string
  active?: boolean
}

type AppShellFrameProps = {
  title: string
  subtitle: string
  navItems: ShellNavItem[]
  children: ReactNode
}

export function AppShellFrame({
  children,
  navItems,
  subtitle,
  title
}: AppShellFrameProps) {
  return (
    <AppShell
      header={{ height: 72 }}
      navbar={{ width: 280, breakpoint: 'sm' }}
      padding="lg"
    >
      <AppShell.Header px="lg">
        <Group h="100%" justify="space-between">
          <Box>
            <Text fw={700}>{title}</Text>
            <Text c="dimmed" size="sm">
              {subtitle}
            </Text>
          </Box>
          <Text c="dimmed" size="sm">
            HugeRouter Console
          </Text>
        </Group>
      </AppShell.Header>
      <AppShell.Navbar p="md">
        <Stack gap="xs">
          {navItems.map((item) => (
            <NavLink
              key={item.href}
              active={item.active}
              description={item.description}
              href={item.href}
              label={item.label}
            />
          ))}
        </Stack>
      </AppShell.Navbar>
      <AppShell.Main>{children}</AppShell.Main>
    </AppShell>
  )
}

