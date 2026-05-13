import { screen } from '@testing-library/react'
import { useQuery } from '@tanstack/react-query'
import { describe, expect, it } from 'vitest'
import { createTestQueryClient, renderWithProviders } from './index'

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

function QueryProbe() {
  const { data } = useQuery({
    queryKey: ['probe'],
    queryFn: () => Promise.resolve('loaded')
  })

  return <div>{data ?? 'pending'}</div>
}

describe('test-utils', () => {
  it('creates query clients with retries disabled', () => {
    const queryClient = createTestQueryClient()

    expect(queryClient.getDefaultOptions().queries?.retry).toBe(false)
    expect(queryClient.getDefaultOptions().mutations?.retry).toBe(false)
  })

  it('renders children with shared providers', async () => {
    renderWithProviders(<QueryProbe />)

    expect(await screen.findByText('loaded')).toBeTruthy()
  })
})
