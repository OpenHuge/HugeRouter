import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { AppShellFrame, EmptyState, PageHeader } from './index'

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => undefined,
    addListener: () => undefined,
    dispatchEvent: () => false,
    removeEventListener: () => undefined,
    removeListener: () => undefined
  })
})

describe('ui-kit', () => {
  it('renders EmptyState fallback action copy', () => {
    render(
      <MantineProvider>
        <EmptyState description="No providers are registered." title="No providers" />
      </MantineProvider>
    )

    expect(screen.getByRole('heading', { name: 'No providers' })).toBeTruthy()
    expect(screen.getByText('Placeholder action')).toBeTruthy()
  })

  it('renders PageHeader actions', () => {
    render(
      <MantineProvider>
        <PageHeader
          actions={<button type="button">Create route</button>}
          description="Inspect the active tenant state."
          title="Overview"
        />
      </MantineProvider>
    )

    expect(screen.getByRole('heading', { name: 'Overview' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Create route' })).toBeTruthy()
  })

  it('renders AppShellFrame navigation and chrome', () => {
    render(
      <MantineProvider>
        <AppShellFrame
          navItems={[
            {
              href: '/app/overview',
              label: 'Overview',
              description: 'Workspace summary',
              active: true
            }
          ]}
          subtitle="Tenant-facing control plane views."
          title="Tenant Workspace"
        >
          <div>Shell body</div>
        </AppShellFrame>
      </MantineProvider>
    )

    expect(screen.getByText('HugeRouter Console')).toBeTruthy()
    expect(screen.getByRole('link', { name: /Overview/ }).getAttribute('href')).toBe('/app/overview')
    expect(screen.getByText('Shell body')).toBeTruthy()
  })
})
