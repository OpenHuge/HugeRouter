import {
  Badge,
  Button,
  Card,
  Checkbox,
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
  type ProviderResourceMutationInput,
} from "../features/control-plane/service";
import { type ProjectSummary } from "../features/control-plane/types";
import {
  ActionStatusNotice,
  FieldErrorText,
} from "../features/control-plane/workflow-ui";

type ProvidersPageData = {
  projects: ProjectSummary[];
  providers: ProviderResource[];
};

type ProviderFormState = {
  authKind: ProviderResourceMutationInput["authKind"];
  budgetPolicyId: string;
  credentialOwnerType: ProviderResourceMutationInput["credentialOwnerType"];
  deploymentScope: ProviderResourceMutationInput["deploymentScope"];
  endpointBaseUrl: string;
  healthState: ProviderResourceMutationInput["healthState"];
  name: string;
  projectId: string;
  providerId: string;
  providerResourceId: string;
  provenanceClass: ProviderResourceMutationInput["provenanceClass"];
  region: string;
  status: ProviderResourceMutationInput["status"];
  supportsJsonMode: boolean;
  supportsStreaming: boolean;
  supportsToolCalling: boolean;
};

type ProviderFormErrors = Partial<Record<keyof ProviderFormState, string>>;

const providerStatusOptions = [
  { label: "active", value: "active" },
  { label: "quarantined", value: "quarantined" },
  { label: "draining", value: "draining" },
  { label: "disabled", value: "disabled" },
  { label: "deleted", value: "deleted" },
] as const;

const providerHealthOptions = [
  { label: "healthy", value: "healthy" },
  { label: "degraded", value: "degraded" },
  { label: "quarantined", value: "quarantined" },
  { label: "draining", value: "draining" },
  { label: "disabled", value: "disabled" },
] as const;

const authKindOptions = [
  { label: "api_key", value: "api_key" },
  {
    label: "oauth_client_credentials",
    value: "oauth_client_credentials",
  },
  { label: "session_broker", value: "session_broker" },
] as const;

const provenanceOptions = [
  { label: "official_api", value: "official_api" },
  { label: "official_gateway", value: "official_gateway" },
  {
    label: "byo_customer_credential",
    value: "byo_customer_credential",
  },
  {
    label: "dedicated_managed_account",
    value: "dedicated_managed_account",
  },
  { label: "shared_brokered_pool", value: "shared_brokered_pool" },
  {
    label: "unofficial_client_channel",
    value: "unofficial_client_channel",
  },
] as const;

const credentialOwnerOptions = [
  { label: "platform", value: "platform" },
  { label: "tenant", value: "tenant" },
  { label: "project", value: "project" },
  { label: "partner", value: "partner" },
] as const;

const deploymentScopeOptions = [
  { label: "shared", value: "shared" },
  { label: "tenant_dedicated", value: "tenant_dedicated" },
  { label: "project_dedicated", value: "project_dedicated" },
] as const;

function createEmptyProviderForm(): ProviderFormState {
  return {
    authKind: "api_key",
    budgetPolicyId: "",
    credentialOwnerType: "platform",
    deploymentScope: "shared",
    endpointBaseUrl: "https://",
    healthState: "healthy",
    name: "",
    projectId: "",
    providerId: "",
    providerResourceId: "",
    provenanceClass: "official_api",
    region: "",
    status: "active",
    supportsJsonMode: true,
    supportsStreaming: true,
    supportsToolCalling: true,
  };
}

function providerToFormState(provider: ProviderResource): ProviderFormState {
  return {
    authKind: provider.auth_kind,
    budgetPolicyId: provider.budget_policy_id ?? "",
    credentialOwnerType: provider.credential_owner_type,
    deploymentScope: provider.deployment_scope,
    endpointBaseUrl: provider.endpoint_base_url,
    healthState: provider.health_state,
    name: provider.name,
    projectId: provider.project_id ?? "",
    providerId: provider.provider_id,
    providerResourceId: provider.provider_resource_id,
    provenanceClass: provider.provenance_class,
    region: provider.region,
    status: provider.status,
    supportsJsonMode: provider.capabilities.supports_json_mode,
    supportsStreaming: provider.capabilities.supports_streaming,
    supportsToolCalling: provider.capabilities.supports_tool_calling,
  };
}

function validateProviderForm(form: ProviderFormState) {
  const errors: ProviderFormErrors = {};

  if (!/^prvrsrc_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.providerResourceId)) {
    errors.providerResourceId =
      "Use an id that starts with prvrsrc_ and contains letters, digits, _ or -.";
  }

  if (!form.name.trim()) {
    errors.name = "Enter a provider resource name.";
  }

  if (!form.providerId.trim()) {
    errors.providerId = "Enter the upstream provider id.";
  }

  if (!form.region.trim()) {
    errors.region = "Enter a deployment region.";
  }

  if (!/^https:\/\/.+/.test(form.endpointBaseUrl.trim())) {
    errors.endpointBaseUrl = "Use an HTTPS endpoint URL.";
  }

  if (form.budgetPolicyId.trim() && !/^budgetpol_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.budgetPolicyId)) {
    errors.budgetPolicyId = "Budget policy ids must start with budgetpol_.";
  }

  return errors;
}

export const Route = createFileRoute("/app/providers")({
  loader: () =>
    loadRouteData(async () => {
      const [providers, projects] = await Promise.all([
        getConsoleDataService().listProviderResources(),
        getConsoleDataService().listProjects(),
      ]);

      return {
        projects,
        providers,
      } satisfies ProvidersPageData;
    }),
  pendingComponent: () => <RouteLoadingState label="Loading providers" />,
  pendingMs: 0,
  component: ProvidersPage,
});

function ProvidersPage() {
  const result = Route.useLoaderData();
  const router = useRouter();
  const [editingProvider, setEditingProvider] = useState<ProviderResource | null>(
    null,
  );
  const [formErrors, setFormErrors] = useState<ProviderFormErrors>({});
  const [formMode, setFormMode] = useState<"create" | "edit" | null>(null);
  const [formState, setFormState] = useState<ProviderFormState>(
    createEmptyProviderForm(),
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusSuccess, setStatusSuccess] = useState<string | null>(null);
  const [disablingProviderId, setDisablingProviderId] = useState<string | null>(
    null,
  );

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Inspect provider resources, health, routing scope, and quarantine state."
          title="Providers"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="Provider inventory could not be loaded from the control-plane service."
          title="Providers unavailable"
        />
      </Stack>
    );
  }

  const { projects, providers } = result.data;
  const projectOptions = [
    { label: "No project scope", value: "" },
    ...projects.map((project) => ({
      label: project.name,
      value: project.id,
    })),
  ];

  function updateField<K extends keyof ProviderFormState>(
    key: K,
    value: ProviderFormState[K],
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
    setEditingProvider(null);
    setFormErrors({});
    setFormMode(null);
    setFormState(createEmptyProviderForm());
  }

  function openCreateForm() {
    setStatusError(null);
    setStatusSuccess(null);
    setEditingProvider(null);
    setFormErrors({});
    setFormMode("create");
    setFormState(createEmptyProviderForm());
  }

  function openEditForm(provider: ProviderResource) {
    setStatusError(null);
    setStatusSuccess(null);
    setEditingProvider(provider);
    setFormErrors({});
    setFormMode("edit");
    setFormState(providerToFormState(provider));
  }

  async function refreshRoute() {
    await router.invalidate();
  }

  async function onSubmit() {
    const errors = validateProviderForm(formState);
    setFormErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);

    const input: ProviderResourceMutationInput = {
      authKind: formState.authKind,
      budgetPolicyId: formState.budgetPolicyId.trim() || undefined,
      capabilities: {
        supportsJsonMode: formState.supportsJsonMode,
        supportsStreaming: formState.supportsStreaming,
        supportsToolCalling: formState.supportsToolCalling,
      },
      credentialOwnerType: formState.credentialOwnerType,
      createdAt: editingProvider?.created_at,
      deploymentScope: formState.deploymentScope,
      endpointBaseUrl: formState.endpointBaseUrl.trim(),
      healthState: formState.healthState,
      name: formState.name.trim(),
      projectId: formState.projectId || undefined,
      providerId: formState.providerId.trim(),
      providerResourceId: formState.providerResourceId.trim(),
      provenanceClass: formState.provenanceClass,
      region: formState.region.trim(),
      status: formState.status,
      version: editingProvider?.version,
    };

    try {
      if (formMode === "edit" && editingProvider) {
        await getConsoleDataService().updateProviderResource(
          editingProvider.provider_resource_id,
          input,
          editingProvider.version,
        );
        setStatusSuccess(`Updated provider resource ${formState.name}.`);
      } else {
        await getConsoleDataService().createProviderResource(input);
        setStatusSuccess(`Created provider resource ${formState.name}.`);
      }

      resetForm();
      await refreshRoute();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(
          error,
          formMode === "edit" ? "provider-update" : "provider-create",
        ),
      );
    } finally {
      setIsSubmitting(false);
    }
  }

  async function onDisable(provider: ProviderResource) {
    setDisablingProviderId(provider.provider_resource_id);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      await getConsoleDataService().disableProviderResource(
        provider.provider_resource_id,
        provider.version,
      );
      setStatusSuccess(`Disabled provider resource ${provider.name}.`);
      await refreshRoute();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "provider-disable"),
      );
    } finally {
      setDisablingProviderId(null);
    }
  }

  return (
    <Stack>
      <PageHeader
        description="Inspect provider resources, health, routing scope, and quarantine state."
        title="Providers"
      />
      <ActionStatusNotice
        error={statusError}
        onDismiss={() => {
          setStatusError(null);
          setStatusSuccess(null);
        }}
        success={statusSuccess}
      />
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>
            {formMode === "edit"
              ? "Edit provider resource"
              : "Create provider resource"}
          </Text>
          <Group>
            {formMode ? (
              <Button
                onClick={resetForm}
                size="sm"
                variant="subtle"
              >
                Cancel
              </Button>
            ) : null}
            {!formMode ? (
              <Button onClick={openCreateForm} size="sm">
                Create provider
              </Button>
            ) : null}
          </Group>
        </Group>
        {formMode ? (
          <Stack>
            <TextInput
              label="Provider resource id"
              onChange={(event) =>
                updateField("providerResourceId", event.currentTarget.value)
              }
              placeholder="prvrsrc_openai_primary"
              value={formState.providerResourceId}
            />
            <FieldErrorText error={formErrors.providerResourceId} />
            <Group grow>
              <TextInput
                label="Display name"
                onChange={(event) => updateField("name", event.currentTarget.value)}
                placeholder="OpenAI Primary"
                value={formState.name}
              />
              <TextInput
                label="Provider id"
                onChange={(event) =>
                  updateField("providerId", event.currentTarget.value)
                }
                placeholder="openai"
                value={formState.providerId}
              />
            </Group>
            <Group grow>
              <FieldErrorText error={formErrors.name} />
              <FieldErrorText error={formErrors.providerId} />
            </Group>
            <Group grow>
              <Select
                data={projectOptions}
                label="Project scope"
                onChange={(value) => updateField("projectId", value ?? "")}
                value={formState.projectId}
              />
              <TextInput
                label="Region"
                onChange={(event) => updateField("region", event.currentTarget.value)}
                placeholder="us-east-1"
                value={formState.region}
              />
            </Group>
            <FieldErrorText error={formErrors.region} />
            <TextInput
              label="Endpoint URL"
              onChange={(event) =>
                updateField("endpointBaseUrl", event.currentTarget.value)
              }
              placeholder="https://api.openai.com/v1"
              value={formState.endpointBaseUrl}
            />
            <FieldErrorText error={formErrors.endpointBaseUrl} />
            <Group grow>
              <Select
                data={provenanceOptions}
                label="Provenance"
                onChange={(value) =>
                  updateField(
                    "provenanceClass",
                    value ?? "official_api",
                  )
                }
                value={formState.provenanceClass}
              />
              <Select
                data={credentialOwnerOptions}
                label="Credential owner"
                onChange={(value) =>
                  updateField(
                    "credentialOwnerType",
                    value ?? "platform",
                  )
                }
                value={formState.credentialOwnerType}
              />
            </Group>
            <Group grow>
              <Select
                data={deploymentScopeOptions}
                label="Deployment scope"
                onChange={(value) =>
                  updateField(
                    "deploymentScope",
                    value ?? "shared",
                  )
                }
                value={formState.deploymentScope}
              />
              <Select
                data={authKindOptions}
                label="Auth kind"
                onChange={(value) =>
                  updateField(
                    "authKind",
                    value ?? "api_key",
                  )
                }
                value={formState.authKind}
              />
            </Group>
            <Group grow>
              <Select
                data={providerHealthOptions}
                label="Health state"
                onChange={(value) =>
                  updateField(
                    "healthState",
                    value ?? "healthy",
                  )
                }
                value={formState.healthState}
              />
              <Select
                data={providerStatusOptions}
                label="Resource status"
                onChange={(value) =>
                  updateField(
                    "status",
                    value ?? "active",
                  )
                }
                value={formState.status}
              />
            </Group>
            <TextInput
              label="Budget policy id"
              onChange={(event) =>
                updateField("budgetPolicyId", event.currentTarget.value)
              }
              placeholder="budgetpol_default"
              value={formState.budgetPolicyId}
            />
            <FieldErrorText error={formErrors.budgetPolicyId} />
            <Stack gap="xs">
              <Text fw={600} size="sm">
                Capabilities
              </Text>
              <Group>
                <Checkbox
                  checked={formState.supportsStreaming}
                  label="streaming"
                  onChange={(event) =>
                    updateField("supportsStreaming", event.currentTarget.checked)
                  }
                />
                <Checkbox
                  checked={formState.supportsToolCalling}
                  label="tool_calling"
                  onChange={(event) =>
                    updateField(
                      "supportsToolCalling",
                      event.currentTarget.checked,
                    )
                  }
                />
                <Checkbox
                  checked={formState.supportsJsonMode}
                  label="json_mode"
                  onChange={(event) =>
                    updateField("supportsJsonMode", event.currentTarget.checked)
                  }
                />
              </Group>
            </Stack>
            <Group justify="flex-end">
              <Button
                loading={isSubmitting}
                onClick={() => void onSubmit()}
              >
                {formMode === "edit" ? "Save provider" : "Create provider"}
              </Button>
            </Group>
          </Stack>
        ) : (
          <Text c="dimmed" size="sm">
            Create a new provider resource, or edit and disable an existing one.
          </Text>
        )}
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Provider inventory</Text>
          <Badge color="blue" variant="light">
            {providers.length} resources
          </Badge>
        </Group>
        {providers.length === 0 ? (
          <EmptyCollectionState
            description="Register a provider resource to begin routing tenant traffic."
            title="No providers"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Name</Table.Th>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Project</Table.Th>
                <Table.Th>Region</Table.Th>
                <Table.Th>Scope</Table.Th>
                <Table.Th>Health</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Version</Table.Th>
                <Table.Th>Actions</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {providers.map((provider) => (
                <Table.Tr key={provider.provider_resource_id}>
                  <Table.Td>{provider.name}</Table.Td>
                  <Table.Td>{provider.provider_id}</Table.Td>
                  <Table.Td>
                    {projects.find((project) => project.id === provider.project_id)
                      ?.name ?? "Unscoped"}
                  </Table.Td>
                  <Table.Td>{provider.region}</Table.Td>
                  <Table.Td>{provider.deployment_scope}</Table.Td>
                  <Table.Td>
                    <Badge
                      color={
                        provider.health_state === "healthy"
                          ? "teal"
                          : provider.health_state === "degraded"
                            ? "yellow"
                            : "red"
                      }
                      variant="light"
                    >
                      {provider.health_state}
                    </Badge>
                  </Table.Td>
                  <Table.Td>
                    <Badge
                      color={provider.status === "active" ? "blue" : "gray"}
                      variant="light"
                    >
                      {provider.status}
                    </Badge>
                  </Table.Td>
                  <Table.Td>{provider.version}</Table.Td>
                  <Table.Td>
                    <Group gap="xs">
                      <Button
                        onClick={() => openEditForm(provider)}
                        size="xs"
                        variant="light"
                      >
                        Edit
                      </Button>
                      <Button
                        color="red"
                        disabled={
                          provider.status === "disabled" ||
                          disablingProviderId === provider.provider_resource_id
                        }
                        loading={
                          disablingProviderId === provider.provider_resource_id
                        }
                        onClick={() => void onDisable(provider)}
                        size="xs"
                        variant="light"
                      >
                        Disable
                      </Button>
                    </Group>
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
