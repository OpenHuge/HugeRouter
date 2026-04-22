import { Outlet, createFileRoute, useLocation, useNavigate } from '@tanstack/react-router'
import { LoginPage } from '../features/auth/LoginPage'
import { parseLoginSearch } from '../features/auth/auth-contract'
import { authSessionQueryOptions } from '../features/auth/auth-queries'
import { getPostLoginDestination, redirectAuthenticatedUsers } from '../features/auth/auth-routing'
import { getQueryClient } from '../lib/query-client'

export const Route = createFileRoute('/login')({
  beforeLoad: async ({ search }) => {
    await redirectAuthenticatedUsers(search.redirect)
  },
  loader: () => getQueryClient().ensureQueryData(authSessionQueryOptions()),
  validateSearch: parseLoginSearch,
  component: LoginRoute
})

function LoginRoute() {
  const navigate = useNavigate()
  const location = useLocation()
  const search = Route.useSearch()
  const sessionEnvelope = Route.useLoaderData()

  if (location.pathname.startsWith('/login/')) {
    return <Outlet />
  }

  return (
    <LoginPage
      onAuthenticated={(state) =>
        navigate({
          to: getPostLoginDestination(state, search.redirect)
        })
      }
      search={search}
      sessionEnvelope={sessionEnvelope}
    />
  )
}
