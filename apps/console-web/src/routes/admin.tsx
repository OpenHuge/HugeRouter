import { Outlet, createFileRoute, useLocation } from '@tanstack/react-router'
import { AppShellFrame } from '@huge-router/ui-kit'
import { AuthenticatedShellContent } from '../features/auth/AuthenticatedShellContent'
import { ensureAuthenticatedSession, requireAdminSession } from '../features/auth/auth-routing'
import { adminNav } from '../lib/navigation'

export const Route = createFileRoute('/admin')({
  beforeLoad: async ({ location }) => {
    const envelope = await ensureAuthenticatedSession(location.href)
    requireAdminSession(envelope)
  },
  component: AdminLayout
})

function AdminLayout() {
  const location = useLocation()

  return (
    <AppShellFrame
      navItems={adminNav.map((item) => ({
        ...item,
        active: location.pathname.startsWith(item.href)
      }))}
      subtitle="Platform administration surfaces aligned with the first control-plane slice."
      title="Admin Console"
    >
      <AuthenticatedShellContent>
        <Outlet />
      </AuthenticatedShellContent>
    </AppShellFrame>
  )
}
