import {
  Outlet,
  createFileRoute,
  redirect,
  useLocation,
} from "@tanstack/react-router";
import { AppShellFrame } from "@huge-router/ui-kit";
import { AuthenticatedShellContent } from "../features/auth/AuthenticatedShellContent";
import { ensureAuthenticatedSession } from "../features/auth/auth-routing";
import { tenantNav } from "../lib/navigation";

function throwRedirect(options: Parameters<typeof redirect>[0]): never {
  throw redirect(options) as unknown as Error;
}

export const Route = createFileRoute("/app")({
  beforeLoad: async ({ location }) => {
    const envelope = await ensureAuthenticatedSession(location.href);

    if (
      envelope.state.kind === "authenticated" &&
      envelope.state.session.user.isPlatformAdmin
    ) {
      throwRedirect({
        to: "/admin/tenants",
      });
    }
  },
  component: AppLayout,
});

function AppLayout() {
  const location = useLocation();

  return (
    <AppShellFrame
      navItems={tenantNav.map((item) => ({
        ...item,
        active: location.pathname.startsWith(item.href),
      }))}
      subtitle="Tenant-facing control-plane views with route-aware navigation and data-backed states."
      title="Tenant Workspace"
    >
      <AuthenticatedShellContent>
        <Outlet />
      </AuthenticatedShellContent>
    </AppShellFrame>
  );
}
