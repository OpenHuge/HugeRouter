import {
  UiChip,
  UiButton,
  UiSurface,
  UiCheckbox,
  UiInline,
  UiSelect,
  UiStack,
  UiDataTable,
  UiText,
  UiTextField,
} from "@huge-router/ui-kit";
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
import type {
  ProjectSummary,
  RoutePolicyView,
  RouteReceiptView,
} from "../features/control-plane/types";
import {
  ActionStatusNotice,
  FieldErrorText,
} from "../features/control-plane/workflow-ui";

type ProvidersPageData = {
  projects: ProjectSummary[];
  providers: ProviderResource[];
  routePolicies: RoutePolicyView[];
  routeReceipts: RouteReceiptView[];
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

  if (
    form.budgetPolicyId.trim() &&
    !/^budgetpol_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.budgetPolicyId)
  ) {
    errors.budgetPolicyId = "Budget policy ids must start with budgetpol_.";
  }

  return errors;
}

export const Route = createFileRoute("/app/providers")({
  loader: () =>
    loadRouteData(async () => {
      const [providers, projects, routePolicies, routeReceipts] =
        await Promise.all([
          getConsoleDataService().listProviderResources(),
          getConsoleDataService().listProjects(),
          getConsoleDataService().listRoutePolicies(),
          getConsoleDataService().listRouteReceipts(),
        ]);

      return {
        projects,
        providers,
        routePolicies,
        routeReceipts,
      } satisfies ProvidersPageData;
    }),
  pendingComponent: () => <RouteLoadingState label="Loading providers" />,
  pendingMs: 0,
  component: ProvidersPage,
});

function ProvidersPage() {
  const result = Route.useLoaderData();
  const router = useRouter();
  const [editingProvider, setEditingProvider] =
    useState<ProviderResource | null>(null);
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
      <UiStack>
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
      </UiStack>
    );
  }

  const { projects, providers, routePolicies, routeReceipts } = result.data;
  const routePolicyNames = new Map(
    routePolicies.map((policy) => [policy.id, policy.name]),
  );
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
    <UiStack>
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
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>
            {formMode === "edit"
              ? "Edit provider resource"
              : "Create provider resource"}
          </UiText>
          <UiInline>
            {formMode ? (
              <UiButton onClick={resetForm} size="sm" variant="subtle">
                Cancel
              </UiButton>
            ) : null}
            {!formMode ? (
              <UiButton onClick={openCreateForm} size="sm">
                Create provider
              </UiButton>
            ) : null}
          </UiInline>
        </UiInline>
        {formMode ? (
          <UiStack>
            <UiTextField
              label="Provider resource id"
              onChange={(event) =>
                updateField("providerResourceId", event.currentTarget.value)
              }
              placeholder="prvrsrc_openai_primary"
              value={formState.providerResourceId}
            />
            <FieldErrorText error={formErrors.providerResourceId} />
            <UiInline grow>
              <UiTextField
                label="Display name"
                onChange={(event) =>
                  updateField("name", event.currentTarget.value)
                }
                placeholder="OpenAI Primary"
                value={formState.name}
              />
              <UiTextField
                label="Provider id"
                onChange={(event) =>
                  updateField("providerId", event.currentTarget.value)
                }
                placeholder="openai"
                value={formState.providerId}
              />
            </UiInline>
            <UiInline grow>
              <FieldErrorText error={formErrors.name} />
              <FieldErrorText error={formErrors.providerId} />
            </UiInline>
            <UiInline grow>
              <UiSelect
                data={projectOptions}
                label="Project scope"
                onChange={(value) => updateField("projectId", value ?? "")}
                value={formState.projectId}
              />
              <UiTextField
                label="Region"
                onChange={(event) =>
                  updateField("region", event.currentTarget.value)
                }
                placeholder="us-east-1"
                value={formState.region}
              />
            </UiInline>
            <FieldErrorText error={formErrors.region} />
            <UiTextField
              label="Endpoint URL"
              onChange={(event) =>
                updateField("endpointBaseUrl", event.currentTarget.value)
              }
              placeholder="https://api.openai.com/v1"
              value={formState.endpointBaseUrl}
            />
            <FieldErrorText error={formErrors.endpointBaseUrl} />
            <UiInline grow>
              <UiSelect
                data={provenanceOptions}
                label="Provenance"
                onChange={(value) =>
                  updateField("provenanceClass", value ?? "official_api")
                }
                value={formState.provenanceClass}
              />
              <UiSelect
                data={credentialOwnerOptions}
                label="Credential owner"
                onChange={(value) =>
                  updateField("credentialOwnerType", value ?? "platform")
                }
                value={formState.credentialOwnerType}
              />
            </UiInline>
            <UiInline grow>
              <UiSelect
                data={deploymentScopeOptions}
                label="Deployment scope"
                onChange={(value) =>
                  updateField("deploymentScope", value ?? "shared")
                }
                value={formState.deploymentScope}
              />
              <UiSelect
                data={authKindOptions}
                label="Auth kind"
                onChange={(value) =>
                  updateField("authKind", value ?? "api_key")
                }
                value={formState.authKind}
              />
            </UiInline>
            <UiInline grow>
              <UiSelect
                data={providerHealthOptions}
                label="Health state"
                onChange={(value) =>
                  updateField("healthState", value ?? "healthy")
                }
                value={formState.healthState}
              />
              <UiSelect
                data={providerStatusOptions}
                label="Resource status"
                onChange={(value) => updateField("status", value ?? "active")}
                value={formState.status}
              />
            </UiInline>
            <UiTextField
              label="Budget policy id"
              onChange={(event) =>
                updateField("budgetPolicyId", event.currentTarget.value)
              }
              placeholder="budgetpol_default"
              value={formState.budgetPolicyId}
            />
            <FieldErrorText error={formErrors.budgetPolicyId} />
            <UiStack gap="xs">
              <UiText fw={600} size="sm">
                Capabilities
              </UiText>
              <UiInline>
                <UiCheckbox
                  checked={formState.supportsStreaming}
                  label="streaming"
                  onChange={(event) =>
                    updateField(
                      "supportsStreaming",
                      event.currentTarget.checked,
                    )
                  }
                />
                <UiCheckbox
                  checked={formState.supportsToolCalling}
                  label="tool_calling"
                  onChange={(event) =>
                    updateField(
                      "supportsToolCalling",
                      event.currentTarget.checked,
                    )
                  }
                />
                <UiCheckbox
                  checked={formState.supportsJsonMode}
                  label="json_mode"
                  onChange={(event) =>
                    updateField("supportsJsonMode", event.currentTarget.checked)
                  }
                />
              </UiInline>
            </UiStack>
            <UiInline justify="flex-end">
              <UiButton loading={isSubmitting} onClick={() => void onSubmit()}>
                {formMode === "edit" ? "Save provider" : "Create provider"}
              </UiButton>
            </UiInline>
          </UiStack>
        ) : (
          <UiText c="dimmed" size="sm">
            Create a new provider resource, or edit and disable an existing one.
          </UiText>
        )}
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Provider inventory</UiText>
          <UiChip color="blue" variant="light">
            {providers.length} resources
          </UiChip>
        </UiInline>
        {providers.length === 0 ? (
          <EmptyCollectionState
            description="Register a provider resource to begin routing tenant traffic."
            title="No providers"
          />
        ) : (
          <UiDataTable striped withTableBorder>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Name</UiDataTable.Th>
                <UiDataTable.Th>Provider</UiDataTable.Th>
                <UiDataTable.Th>Project</UiDataTable.Th>
                <UiDataTable.Th>Region</UiDataTable.Th>
                <UiDataTable.Th>Scope</UiDataTable.Th>
                <UiDataTable.Th>Protocols</UiDataTable.Th>
                <UiDataTable.Th>Capabilities</UiDataTable.Th>
                <UiDataTable.Th>Health</UiDataTable.Th>
                <UiDataTable.Th>Status</UiDataTable.Th>
                <UiDataTable.Th>Latest route signal</UiDataTable.Th>
                <UiDataTable.Th>Version</UiDataTable.Th>
                <UiDataTable.Th>Actions</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {providers.map((provider) => (
                <UiDataTable.Tr key={provider.provider_resource_id}>
                  <UiDataTable.Td>{provider.name}</UiDataTable.Td>
                  <UiDataTable.Td>{provider.provider_id}</UiDataTable.Td>
                  <UiDataTable.Td>
                    {projects.find(
                      (project) => project.id === provider.project_id,
                    )?.name ?? "Unscoped"}
                  </UiDataTable.Td>
                  <UiDataTable.Td>{provider.region}</UiDataTable.Td>
                  <UiDataTable.Td>{provider.deployment_scope}</UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiText size="sm">
                      {provider.supported_protocol_families.join(", ") ||
                        "None"}
                    </UiText>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiText size="sm">
                      {providerCapabilityLabels(provider).join(", ") || "None"}
                    </UiText>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip
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
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip
                      color={provider.status === "active" ? "blue" : "gray"}
                      variant="light"
                    >
                      {provider.status}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiText c="dimmed" size="sm">
                      {latestSignalForProvider(
                        provider,
                        routeReceipts,
                        routePolicyNames,
                      ) ?? "No recent receipt"}
                    </UiText>
                  </UiDataTable.Td>
                  <UiDataTable.Td>{provider.version}</UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiInline gap="xs">
                      <UiButton
                        onClick={() => openEditForm(provider)}
                        size="xs"
                        variant="light"
                      >
                        Edit
                      </UiButton>
                      <UiButton
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
                      </UiButton>
                    </UiInline>
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

function providerCapabilityLabels(provider: ProviderResource) {
  return [
    provider.capabilities.supports_streaming ? "streaming" : null,
    provider.capabilities.supports_tool_calling ? "tool_calling" : null,
    provider.capabilities.supports_json_mode ? "json_mode" : null,
    provider.capabilities.supports_realtime ? "realtime" : null,
    provider.capabilities.supports_response_model_metadata
      ? "response_model_metadata"
      : null,
  ].filter(Boolean) as string[];
}

function latestSignalForProvider(
  provider: ProviderResource,
  routeReceipts: RouteReceiptView[],
  routePolicyNames: Map<string, string>,
) {
  const receipt = routeReceipts.find(
    (candidate) =>
      candidate.selectedTargetId === provider.provider_resource_id ||
      candidate.excludedTargets.some(
        (target) =>
          target.provider_resource_id === provider.provider_resource_id,
      ),
  );

  if (!receipt) {
    return null;
  }

  if (receipt.selectedTargetId === provider.provider_resource_id) {
    return `${routePolicyNames.get(receipt.routePolicyId) ?? receipt.routeName}: selected`;
  }

  return `${routePolicyNames.get(receipt.routePolicyId) ?? receipt.routeName}: ${receipt.excludedTargets[0]?.reason_code ?? "excluded"}`;
}
