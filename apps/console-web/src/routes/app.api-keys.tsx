import {
  UiAlert,
  UiChip,
  UiButton,
  UiSurface,
  UiInline,
  UiSelect,
  UiStack,
  UiDataTable,
  UiText,
  UiTextField,
} from "@huge-router/ui-kit";
import { useEffect, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import type { ProviderResource } from "@huge-router/ts-shared-schema";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import {
  getConsoleDataService,
  getControlPlaneActionErrorMessage,
} from "../features/control-plane/service";
import type { ApiKeyView } from "../features/control-plane/types";
import {
  ActionStatusNotice,
  FieldErrorText,
} from "../features/control-plane/workflow-ui";

type ApiKeysPageData = {
  apiKeys: ApiKeyView[];
  providers: ProviderResource[];
};

type ApiKeyFormState = {
  apiKey: string;
  displayName: string;
  providerResourceId: string;
};

type ApiKeyFormErrors = Partial<Record<keyof ApiKeyFormState, string>>;

type SecretPreview = {
  apiKeyId: string;
  displayName: string;
  keyPrefix: string;
  secretValue: string;
};

function createEmptyApiKeyForm(): ApiKeyFormState {
  return {
    apiKey: "",
    displayName: "",
    providerResourceId: "",
  };
}

function validateApiKeyForm(form: ApiKeyFormState) {
  const errors: ApiKeyFormErrors = {};

  if (!form.displayName.trim()) {
    errors.displayName = "Enter an API key label.";
  }

  if (!form.providerResourceId) {
    errors.providerResourceId = "Select a provider resource.";
  }

  if (form.apiKey.trim().length < 8) {
    errors.apiKey = "Enter the full secret value before saving.";
  }

  return errors;
}

export const Route = createFileRoute("/app/api-keys")({
  loader: () =>
    loadRouteData(async () => {
      const [apiKeys, providers] = await Promise.all([
        getConsoleDataService().listApiKeys(),
        getConsoleDataService().listProviderResources(),
      ]);

      return {
        apiKeys,
        providers,
      } satisfies ApiKeysPageData;
    }),
  pendingComponent: () => <RouteLoadingState label="Loading API keys" />,
  pendingMs: 0,
  component: ApiKeysPage,
});

function ApiKeysPage() {
  const result = Route.useLoaderData();
  const [formErrors, setFormErrors] = useState<ApiKeyFormErrors>({});
  const [formOpen, setFormOpen] = useState(false);
  const [formState, setFormState] = useState<ApiKeyFormState>(
    createEmptyApiKeyForm(),
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [revokingApiKeyId, setRevokingApiKeyId] = useState<string | null>(null);
  const [secretPreview, setSecretPreview] = useState<SecretPreview | null>(
    null,
  );
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusSuccess, setStatusSuccess] = useState<string | null>(null);

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Manage API key lifecycle and observe key activity at tenant level."
          title="API Keys"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="API keys could not be loaded from the control-plane service."
          title="API keys unavailable"
        />
      </UiStack>
    );
  }

  const { apiKeys: loadedApiKeys, providers } = result.data;
  const [apiKeys, setApiKeys] = useState<ApiKeyView[]>(loadedApiKeys);

  useEffect(() => {
    setApiKeys(loadedApiKeys);
  }, [loadedApiKeys]);

  function openCreateForm() {
    setFormErrors({});
    setFormOpen(true);
    setFormState({
      ...createEmptyApiKeyForm(),
      providerResourceId: providers[0]?.provider_resource_id ?? "",
    });
  }

  function updateField<K extends keyof ApiKeyFormState>(
    key: K,
    value: ApiKeyFormState[K],
  ) {
    setFormState((current) => ({
      ...current,
      [key]: value,
    }));
    setFormErrors((current) => ({
      ...current,
      [key]: undefined,
    }));
  }

  function resetForm() {
    setFormErrors({});
    setFormOpen(false);
    setFormState(createEmptyApiKeyForm());
  }

  async function onCreateApiKey() {
    const errors = validateApiKeyForm(formState);
    setFormErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      const created = await getConsoleDataService().createApiKey({
        apiKey: formState.apiKey.trim(),
        displayName: formState.displayName.trim(),
        providerResourceId: formState.providerResourceId,
      });

      setSecretPreview({
        apiKeyId: created.apiKeyId,
        displayName: created.displayName,
        keyPrefix: created.keyPrefix,
        secretValue: formState.apiKey.trim(),
      });
      setStatusSuccess(`Created API key ${created.displayName}.`);
      resetForm();
      setApiKeys(await getConsoleDataService().listApiKeys());
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "api-key-create"),
      );
    } finally {
      setIsSubmitting(false);
    }
  }

  async function onRevoke(apiKeyId: string, version: number) {
    setRevokingApiKeyId(apiKeyId);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      await getConsoleDataService().revokeApiKey(apiKeyId, version);
      setStatusSuccess(`Revoked API key ${apiKeyId}.`);
      setApiKeys(await getConsoleDataService().listApiKeys());
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "api-key-revoke"),
      );
    } finally {
      setRevokingApiKeyId(null);
    }
  }

  return (
    <UiStack>
      <PageHeader
        description="Manage API key lifecycle and observe key activity at tenant level."
        title="API Keys"
      />
      <ActionStatusNotice
        error={statusError}
        onDismiss={() => {
          setStatusError(null);
          setStatusSuccess(null);
        }}
        success={statusSuccess}
      />
      {secretPreview ? (
        <UiAlert
          color="yellow"
          radius="md"
          title="Save this secret now"
          variant="light"
        >
          <UiStack gap="xs">
            <UiText size="sm">
              The secret value is shown only once. Save it before you dismiss
              this notice.
            </UiText>
            <UiText fw={700} size="sm">
              {secretPreview.displayName}
            </UiText>
            <UiText size="sm">Prefix: {secretPreview.keyPrefix}</UiText>
            <UiText size="sm">Secret: {secretPreview.secretValue}</UiText>
            <UiInline justify="flex-end">
              <UiButton
                onClick={() => setSecretPreview(null)}
                size="xs"
                variant="light"
              >
                Return to list
              </UiButton>
            </UiInline>
          </UiStack>
        </UiAlert>
      ) : null}
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Create API key</UiText>
          <UiButton
            onClick={() => {
              if (formOpen) {
                resetForm();
              } else {
                openCreateForm();
              }
            }}
            size="sm"
            variant={formOpen ? "light" : "filled"}
          >
            {formOpen ? "Hide form" : "Create API key"}
          </UiButton>
        </UiInline>
        {formOpen ? (
          <UiStack>
            <UiTextField
              label="Display name"
              onChange={(event) =>
                updateField("displayName", event.currentTarget.value)
              }
              placeholder="Acme Primary Key"
              value={formState.displayName}
            />
            <FieldErrorText error={formErrors.displayName} />
            <UiSelect
              data={providers.map((provider) => ({
                label: `${provider.name} (${provider.provider_resource_id})`,
                value: provider.provider_resource_id,
              }))}
              label="Provider resource"
              onChange={(value) =>
                updateField("providerResourceId", value ?? "")
              }
              value={formState.providerResourceId}
            />
            <FieldErrorText error={formErrors.providerResourceId} />
            <UiTextField
              label="Secret value"
              onChange={(event) =>
                updateField("apiKey", event.currentTarget.value)
              }
              placeholder="akp_test_very_secret"
              value={formState.apiKey}
            />
            <FieldErrorText error={formErrors.apiKey} />
            <UiInline justify="flex-end">
              <UiButton
                loading={isSubmitting}
                onClick={() => void onCreateApiKey()}
              >
                Save API key
              </UiButton>
            </UiInline>
          </UiStack>
        ) : (
          <UiText c="dimmed" size="sm">
            Create a key, verify the one-time secret preview, then return to the
            list view.
          </UiText>
        )}
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>API keys</UiText>
          <UiChip color="blue" variant="light">
            {apiKeys.length} keys
          </UiChip>
        </UiInline>
        {apiKeys.length === 0 ? (
          <EmptyCollectionState
            description="No API keys are available for this tenant."
            title="No API keys"
          />
        ) : (
          <UiDataTable striped withTableBorder>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Display name</UiDataTable.Th>
                <UiDataTable.Th>Key prefix</UiDataTable.Th>
                <UiDataTable.Th>Provider</UiDataTable.Th>
                <UiDataTable.Th>Status</UiDataTable.Th>
                <UiDataTable.Th>Created</UiDataTable.Th>
                <UiDataTable.Th>Updated</UiDataTable.Th>
                <UiDataTable.Th>Actions</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {apiKeys.map((key) => (
                <UiDataTable.Tr key={key.apiKeyId}>
                  <UiDataTable.Td>{key.displayName}</UiDataTable.Td>
                  <UiDataTable.Td>{key.keyPrefix}</UiDataTable.Td>
                  <UiDataTable.Td>
                    {providers.find(
                      (provider) =>
                        provider.provider_resource_id ===
                        key.providerResourceId,
                    )?.name ?? key.providerResourceId}
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip
                      color={key.isActive ? "teal" : "gray"}
                      variant="light"
                    >
                      {key.isActive ? "Active" : "Revoked"}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>{key.createdAt}</UiDataTable.Td>
                  <UiDataTable.Td>{key.updatedAt}</UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiButton
                      disabled={
                        !key.canRevoke ||
                        !key.isActive ||
                        revokingApiKeyId === key.apiKeyId
                      }
                      loading={revokingApiKeyId === key.apiKeyId}
                      onClick={() => void onRevoke(key.apiKeyId, key.version)}
                      size="sm"
                      variant="light"
                    >
                      Revoke
                    </UiButton>
                  </UiDataTable.Td>
                </UiDataTable.Tr>
              ))}
            </UiDataTable.Tbody>
          </UiDataTable>
        )}
      </UiSurface>
    </UiStack>
  );
}
