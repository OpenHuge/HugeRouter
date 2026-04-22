import {
  ControlPlaneClientError,
  createControlPlaneClient,
} from "@huge-router/ts-api-client";
import type {
  AuthLoginResult,
  AuthSession,
  AuthSessionResponse,
  AuthProvidersResponse,
  OAuthProvider,
} from "@huge-router/ts-shared-schema";
import {
  type EmailLoginCompleteResult,
  type AuthCallbackInput,
  type AuthCallbackResult,
  type AuthProvider,
  type AuthSessionEnvelope,
  type EmailLoginCompleteInput,
  type EmailLoginInput,
  type EmailLoginStartResult,
  type LogoutResult,
  type ProviderLoginStartInput,
  type ProviderLoginStartResult,
  type RequestMeta,
  authCallbackInputSchema,
  emailLoginCompleteResultSchema,
  emailLoginInputSchema,
  getDefaultProviderAvailability,
  providerLoginStartInputSchema,
} from "./auth-contract";

type AuthApiResult<TData> = {
  data: TData;
  meta: RequestMeta;
};

const AUTH_BASE_URL = import.meta.env.VITE_CONTROL_PLANE_BASE_URL
  ? String(import.meta.env.VITE_CONTROL_PLANE_BASE_URL)
  : "";

export class AuthApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly code?: string,
    readonly meta: RequestMeta = {},
  ) {
    super(message);
    this.name = "AuthApiError";
  }
}

const client = createControlPlaneClient({
  baseUrl: AUTH_BASE_URL,
});

function mapProviderAvailability(
  response: AuthProvidersResponse,
): ReturnType<typeof getDefaultProviderAvailability> {
  return response.providers.map((provider) => ({
    enabled: provider.enabled,
    hidden: false,
    provider: provider.provider,
    reason: provider.reasonCode ?? undefined,
  }));
}

function mapAuthenticatedSession(
  session: AuthSession,
  availableProviders: ReturnType<typeof getDefaultProviderAvailability>,
): AuthSessionEnvelope {
  const memberships = session.memberships.map((membership) => ({
    role: membership.role,
    tenantId: membership.tenant.id,
    tenantName: membership.tenant.displayName,
    tenantSlug: membership.tenant.slug,
  }));
  const activeTenant =
    memberships.find(
      (membership) => membership.tenantId === session.activeTenantId,
    ) ?? null;
  const isPlatformAdmin =
    session.activeTenantId == null ||
    activeTenant?.tenantSlug === "platform-admin";

  return {
    state: {
      availableProviders,
      kind: "authenticated",
      session: {
        activeTenant,
        expiresAt: session.expiresAt,
        memberships,
        sessionId: session.sessionId,
        user: {
          avatarUrl: session.user.avatarUrl ?? undefined,
          displayName: session.user.displayName,
          email: session.user.primaryEmail ?? "unknown",
          id: session.user.userId,
          isPlatformAdmin,
        },
      },
    },
  };
}

function mapSessionResponse(
  response: AuthSessionResponse,
  availableProviders: ReturnType<typeof getDefaultProviderAvailability>,
): AuthSessionEnvelope {
  if (!response.session) {
    return {
      state: {
        availableProviders,
        kind: "anonymous",
      },
    };
  }

  return mapAuthenticatedSession(response.session, availableProviders);
}

function mapAuthResult(
  provider: AuthProvider,
  response: AuthLoginResult,
): AuthCallbackResult {
  const authenticatedState = mapAuthenticatedSession(
    response.session,
    getDefaultProviderAvailability(),
  ).state;

  if (authenticatedState.kind !== "authenticated") {
    throw new AuthApiError(
      "HugeRouter auth callback did not create a session.",
      500,
    );
  }

  return {
    message: "HugeRouter session created successfully.",
    outcome: "authenticated",
    provider,
    state: authenticatedState,
  };
}

function mapEmailCompletionResult(
  response: AuthLoginResult,
): EmailLoginCompleteResult {
  const authenticatedState = mapAuthenticatedSession(
    response.session,
    getDefaultProviderAvailability(),
  ).state;

  if (authenticatedState.kind !== "authenticated") {
    throw new AuthApiError(
      "HugeRouter email login did not create a session.",
      500,
    );
  }

  return emailLoginCompleteResultSchema.parse({
    message: "HugeRouter session created successfully.",
    outcome: "authenticated",
    state: authenticatedState,
  });
}

function toAuthApiError(error: unknown): AuthApiError {
  if (error instanceof ControlPlaneClientError) {
    return new AuthApiError(
      error.message,
      error.status,
      error.code,
      error.meta,
    );
  }

  if (error instanceof Error) {
    return new AuthApiError(error.message, 500);
  }

  return new AuthApiError("HugeRouter auth request failed.", 500);
}

export type ConsoleAuthClient = ReturnType<typeof createConsoleAuthClient>;

export function createConsoleAuthClient() {
  return {
    async completeAuthCallback(
      provider: AuthProvider,
      input: AuthCallbackInput,
    ): Promise<AuthApiResult<AuthCallbackResult>> {
      try {
        const payload = authCallbackInputSchema.parse(input);
        const oauthProvider = provider as OAuthProvider;
        const response = await client.completeOAuthLogin(oauthProvider, {
          code: payload.code ?? "",
          state: payload.state ?? "",
        });

        return {
          data: mapAuthResult(provider, response),
          meta: {},
        };
      } catch (error) {
        throw toAuthApiError(error);
      }
    },
    async getSession(): Promise<AuthApiResult<AuthSessionEnvelope>> {
      try {
        const [session, providers] = await Promise.all([
          client.getCurrentSession(),
          client.getAuthProviders(),
        ]);
        const availableProviders = mapProviderAvailability(providers);

        return {
          data: mapSessionResponse(session, availableProviders),
          meta: {},
        };
      } catch (error) {
        throw toAuthApiError(error);
      }
    },
    async logout(): Promise<AuthApiResult<LogoutResult>> {
      try {
        await client.logout();

        return {
          data: {
            outcome: "signed_out",
          },
          meta: {},
        };
      } catch (error) {
        throw toAuthApiError(error);
      }
    },
    async startEmailLogin(
      input: EmailLoginInput,
    ): Promise<AuthApiResult<EmailLoginStartResult>> {
      try {
        const payload = emailLoginInputSchema.parse(input);
        const response = await client.startEmailLogin(payload);

        return {
          data: {
            codeHint: response.codeHint,
            email: payload.email,
            expiresAt: response.expiresAt,
            flowId: response.flowId,
            message:
              response.verificationMode === "one_time_code"
                ? `A one-time code was sent to ${payload.email}.`
                : `A magic link was sent to ${payload.email}.`,
            outcome: "email_sent",
          },
          meta: {},
        };
      } catch (error) {
        throw toAuthApiError(error);
      }
    },
    async completeEmailLogin(
      input: EmailLoginCompleteInput,
    ): Promise<AuthApiResult<EmailLoginCompleteResult>> {
      try {
        const response = await client.completeEmailLogin(input);

        return {
          data: mapEmailCompletionResult(response),
          meta: {},
        };
      } catch (error) {
        throw toAuthApiError(error);
      }
    },
    async startProviderLogin(
      provider: AuthProvider,
      input: ProviderLoginStartInput,
    ): Promise<AuthApiResult<ProviderLoginStartResult>> {
      try {
        const payload = providerLoginStartInputSchema.parse(input);
        const response = await client.startOAuthLogin(
          provider as OAuthProvider,
          payload,
        );

        return {
          data: {
            authorizationUrl: response.authorizationUrl,
            outcome: "redirect",
          },
          meta: {},
        };
      } catch (error) {
        throw toAuthApiError(error);
      }
    },
  };
}
