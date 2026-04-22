import { Outlet, createFileRoute, useLocation } from '@tanstack/react-router'
import { AppShellFrame } from '@huge-router/ui-kit'
import { tenantNav } from '../lib/navigation'

export const Route = createFileRoute('/app')({
  component: AppLayout
})

function AppLayout() {
  const location = useLocation()

  return (
    <AppShellFrame
      navItems={tenantNav.map((item) => ({
        ...item,
        active: location.pathname.startsWith(item.href)
      }))}
      subtitle="Tenant-facing console shell with shared providers and route-aware navigation."
      title="Tenant Workspace"
    >
      <Outlet />
    </AppShellFrame>
  )
}

