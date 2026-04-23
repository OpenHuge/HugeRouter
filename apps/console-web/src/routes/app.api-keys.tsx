import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Select,
  Stack,
  Table,
  Text,
  TextInput,
} from "@mantine/core";
import { useState } from "react";
import { createFileRoute, useRouter } from "@tanstack/react-router";
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
  const router = useRouter();
  const [formErrors, setFormErrors] = useState<ApiKeyFormErrors>({});
  const [formOpen, setFormOpen] = useState(false);
  const [formState, setFormState] = useState<ApiKeyFormState>(
    createEmptyApiKeyForm(),
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [revokingApiKeyId, setRevokingApiKeyId] = useState<string | null>(null);
  const [secretPreview, setSecretPreview] = useState<SecretPreview | null>(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusSuccess, setStatusSuccess] = useState<string | null>(null);

  if (!result || result.state === "error") {
    return (
      <Stack>
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
      </Stack>
    );
  }

  const { apiKeys, providers } = result.data;

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
      await router.invalidate();
    } catch (error) {
      setStatusError(getControlPlaneActionErrorMessage(error, "api-key-create"));
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
      await router.invalidate();
    } catch (error) {
      setStatusError(getControlPlaneActionErrorMessage(error, "api-key-revoke"));
    } finally {
      setRevokingApiKeyId(null);
    }
  }

  return (
    <Stack>
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
        <Alert color="yellow" radius="md" title="Save this secret now" variant="light">
          <Stack gap="xs">
            <Text size="sm">
              The secret value is shown only once. Save it before you dismiss this notice.
            </Text>
            <Text fw={700} size="sm">
              {secretPreview.displayName}
            </Text>
            <Text size="sm">Prefix: {secretPreview.keyPrefix}</Text>
            <Text size="sm">Secret: {secretPreview.secretValue}</Text>
            <Group justify="flex-end">
              <Button onClick={() => setSecretPreview(null)} size="xs" variant="light">
                Return to list
              </Button>
            </Group>
          </Stack>
        </Alert>
      ) : null}
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Create API key</Text>
          <Button
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
          </Button>
        </Group>
        {formOpen ? (
          <Stack>
            <TextInput
              label="Display name"
              onChange={(event) =>
                updateField("displayName", event.currentTarget.value)
              }
              placeholder="Acme Primary Key"
              value={formState.displayName}
            />
            <FieldErrorText error={formErrors.displayName} />
            <Select
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
            <TextInput
              label="Secret value"
              onChange={(event) => updateField("apiKey", event.currentTarget.value)}
              placeholder="akp_test_very_secret"
              value={formState.apiKey}
            />
            <FieldErrorText error={formErrors.apiKey} />
            <Group justify="flex-end">
              <Button loading={isSubmitting} onClick={() => void onCreateApiKey()}>
                Save API key
              </Button>
            </Group>
          </Stack>
        ) : (
          <Text c="dimmed" size="sm">
            Create a key, verify the one-time secret preview, then return to the list view.
          </Text>
        )}
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>API keys</Text>
          <Badge color="blue" variant="light">
            {apiKeys.length} keys
          </Badge>
        </Group>
        {apiKeys.length === 0 ? (
          <EmptyCollectionState
            description="No API keys are available for this tenant."
            title="No API keys"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Display name</Table.Th>
                <Table.Th>Key prefix</Table.Th>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Created</Table.Th>
                <Table.Th>Updated</Table.Th>
                <Table.Th>Actions</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {apiKeys.map((key) => (
                <Table.Tr key={key.apiKeyId}>
                  <Table.Td>{key.displayName}</Table.Td>
                  <Table.Td>{key.keyPrefix}</Table.Td>
                  <Table.Td>
                    {providers.find(
                      (provider) =>
                        provider.provider_resource_id === key.providerResourceId,
                    )?.name ?? key.providerResourceId}
                  </Table.Td>
                  <Table.Td>
                    <Badge color={key.isActive ? "teal" : "gray"} variant="light">
                      {key.isActive ? "Active" : "Revoked"}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{key.createdAt}</Table.Td>
                  <Table.Td>{key.updatedAt}</Table.Td>
                  <Table.Td>
                    <Button
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
                    </Button>
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Card>
    </Stack>
  );
}
