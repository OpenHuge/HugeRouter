import { redirect } from "@tanstack/react-router";
import { getQueryClient } from "../../lib/query-client";
import type {
  AuthSessionEnvelope,
  AuthSessionState,
  LoginReason,
} from "./auth-contract";
import { loadAuthSession } from "./auth-queries";

function throwRedirect(options: Parameters<typeof redirect>[0]): never {
  throw redirect(options) as unknown as Error;
}

export function getPostLoginDestination(
  state: Extract<AuthSessionState, { kind: "authenticated" }>,
  preferredRedirect?: string,
) {
  if (preferredRedirect) {
    return preferredRedirect;
  }

  if (state.session.user.isPlatformAdmin) {
    return "/admin/tenants";
  }

  if (state.session.activeTenant) {
    return "/app/overview";
  }

  return "/app/overview";
}

export function getLoginReasonForState(
  state: AuthSessionState,
): LoginReason | undefined {
  if (state.kind === "tenant_access_denied") {
    return "tenant-denied";
  }

  if (state.kind === "tenant_selection_required") {
    return "tenant-selection";
  }

  return state.kind === "anonymous" ? "session-expired" : undefined;
}

export async function ensureAuthenticatedSession(locationHref: string) {
  const envelope = await loadAuthSession(getQueryClient());

  if (envelope.state.kind === "authenticated") {
    return envelope;
  }

  throwRedirect({
    search: {
      reason: getLoginReasonForState(envelope.state),
      ...(locationHref ? { redirect: locationHref } : {}),
    } as never,
    to: "/login",
  });
}

export async function redirectAuthenticatedUsers(preferredRedirect?: string) {
  const envelope = await loadAuthSession(getQueryClient());

  if (envelope.state.kind === "authenticated") {
    throwRedirect({
      to: getPostLoginDestination(envelope.state, preferredRedirect),
    });
  }

  return envelope;
}

export function requireAdminSession(
  envelope: AuthSessionEnvelope,
): Extract<AuthSessionState, { kind: "authenticated" }> {
  if (envelope.state.kind !== "authenticated") {
    throw new Error("Admin routes require an authenticated session.");
  }

  if (!envelope.state.session.user.isPlatformAdmin) {
    throwRedirect({
      to: "/app/overview",
    });
  }

  return envelope.state;
}
