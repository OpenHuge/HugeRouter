import { Outlet, createFileRoute, useLocation } from '@tanstack/react-router'
import { AppShellFrame } from '@huge-router/ui-kit'
import { adminNav } from '../lib/navigation'

export const Route = createFileRoute('/admin')({
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
      subtitle="Platform administration shell aligned with the future control-plane surface."
      title="Admin Console"
    >
      <Outlet />
    </AppShellFrame>
  )
}

