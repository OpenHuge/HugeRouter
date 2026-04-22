import { screen } from '@testing-library/react'
import { renderWithProviders } from '@huge-router/test-utils'
import { describe, expect, it } from 'vitest'
import { LoginPage } from '../features/auth/LoginPage'

describe('LoginPage', () => {
  it('renders the login options for email and social providers', () => {
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
        name: 'Continue with Email'
      })
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', {
        name: 'Continue with GitHub'
      })
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', {
        name: 'Continue with Google'
      })
    ).toBeInTheDocument()
    expect(
      screen.getByRole('button', {
        name: 'Continue with WeChat'
      })
    ).toBeInTheDocument()
  })
})
