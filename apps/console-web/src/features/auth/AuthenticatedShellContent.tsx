import {
  UiChip,
  UiButton,
  UiSurface,
  UiInline,
  UiStack,
  UiText,
} from "@huge-router/ui-kit";
import type { ReactNode } from "react";
import { useLogoutMutation } from "./auth-queries";
import { useRequestContext } from "../../start";

type AuthenticatedShellContentProps = {
  children: ReactNode;
};

export function AuthenticatedShellContent({
  children,
}: AuthenticatedShellContentProps) {
  const requestContext = useRequestContext();
  const logoutMutation = useLogoutMutation();

  return (
    <UiStack gap="lg">
      {requestContext.session.kind === "authenticated" ? (
        <UiSurface padding="lg" radius="md" shadow="sm">
          <UiInline justify="space-between" wrap="wrap">
            <div>
              <UiText fw={700}>
                {requestContext.session.session.user.displayName}
              </UiText>
              <UiText c="dimmed" size="sm">
                {requestContext.session.session.user.email}
              </UiText>
            </div>
            <UiInline gap="sm">
              <UiChip color="blue" variant="light">
                {requestContext.session.session.user.isPlatformAdmin
                  ? "platform-admin"
                  : (requestContext.session.session.activeTenant?.tenantSlug ??
                    "tenant")}
              </UiChip>
              <UiButton
                loading={logoutMutation.isPending}
                onClick={() => void logoutMutation.mutateAsync()}
                variant="light"
              >
                Sign out
              </UiButton>
            </UiInline>
          </UiInline>
        </UiSurface>
      ) : null}
      {children}
    </UiStack>
  );
}
