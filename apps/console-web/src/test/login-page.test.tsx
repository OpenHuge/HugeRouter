import { screen } from '@testing-library/react'
import { renderWithProviders } from '@huge-router/test-utils'
import { describe, expect, it } from 'vitest'
import { LoginPage } from '../features/auth/LoginPage'

describe('LoginPage', () => {
  it('renders the placeholder authentication shell', () => {
    renderWithProviders(<LoginPage />)

    expect(
      screen.getByRole('heading', {
        name: 'Sign in'
      })
    ).toBeInTheDocument()
    expect(screen.getByLabelText('Workspace')).toHaveAttribute(
      'placeholder',
      'platform-admin'
    )
    expect(
      screen.getByRole('button', {
        name: 'Placeholder authentication'
      })
    ).toBeInTheDocument()
  })
})
