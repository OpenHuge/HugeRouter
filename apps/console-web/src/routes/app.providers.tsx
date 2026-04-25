import {
  Badge,
  Button,
  Card,
  Checkbox,
  FileInput,
  Group,
  NumberInput,
  Select,
  Switch,
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
  type OAuthCarpoolMutationInput,
  type OAuthCarpoolView,
  type OAuthSharingLeaseMutationInput,
  type OAuthSharingLeaseView,
  type OAuthSharingUsageView,
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
  carpools: OAuthCarpoolView[];
  leases: OAuthSharingLeaseView[];
  sharingUsage: OAuthSharingUsageView;
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

type CodexAuthUploadState = {
  displayName: string;
  endpointBaseUrl: string;
  file: File | null;
  projectId: string;
  providerResourceId: string;
  region: string;
};

type CodexAuthUploadErrors = Partial<
  Record<keyof CodexAuthUploadState, string>
>;

type SharingLeaseFormState = {
  borrowerWorkspaceId: string;
  expiresAt: string;
  leaseId: string;
  maxConcurrentRuns: number;
  policy: OAuthSharingLeaseMutationInput["policy"];
  poolId: string;
  provider: OAuthSharingLeaseMutationInput["provider"];
  status: OAuthSharingLeaseMutationInput["status"];
  turnBudget: number;
};

type CarpoolFormState = {
  carpoolId: string;
  enabled: boolean;
  memberWorkspaceIds: string;
  name: string;
  perMemberConcurrencyLimit: number;
  perMemberTurnBudget: number;
  poolIds: string;
  provider: OAuthCarpoolMutationInput["provider"];
  strategy: OAuthCarpoolMutationInput["strategy"];
};

type SharingFormErrors = Partial<
  Record<keyof SharingLeaseFormState | keyof CarpoolFormState, string>
>;

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

function createEmptyCodexAuthUpload(): CodexAuthUploadState {
  return {
    displayName: "",
    endpointBaseUrl: "https://",
    file: null,
    projectId: "",
    providerResourceId: "",
    region: "global",
  };
}

function defaultSharingExpiry() {
  return new Date(Date.now() + 1000 * 60 * 60 * 24 * 30).toISOString();
}

function createEmptySharingLeaseForm(): SharingLeaseFormState {
  return {
    borrowerWorkspaceId: "tenant_acme",
    expiresAt: defaultSharingExpiry(),
    leaseId: "",
    maxConcurrentRuns: 1,
    policy: "fair_share",
    poolId: "",
    provider: "codex",
    status: "active",
    turnBudget: 20,
  };
}

function createEmptyCarpoolForm(): CarpoolFormState {
  return {
    carpoolId: "",
    enabled: true,
    memberWorkspaceIds: "tenant_acme",
    name: "",
    perMemberConcurrencyLimit: 2,
    perMemberTurnBudget: 50,
    poolIds: "",
    provider: "codex",
    strategy: "fair_share",
  };
}

function splitCsv(value: string) {
  return value
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
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
      const [
        providers,
        projects,
        routePolicies,
        routeReceipts,
        leases,
        carpools,
        sharingUsage,
      ] = await Promise.all([
        getConsoleDataService().listProviderResources(),
        getConsoleDataService().listProjects(),
        getConsoleDataService().listRoutePolicies(),
        getConsoleDataService().listRouteReceipts(),
        getConsoleDataService().listOAuthSharingLeases(),
        getConsoleDataService().listOAuthCarpools(),
        getConsoleDataService().readOAuthSharingUsage(),
      ]);

      return {
        carpools,
        leases,
        projects,
        providers,
        routePolicies,
        routeReceipts,
        sharingUsage,
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
  const [codexAuthUpload, setCodexAuthUpload] =
    useState<CodexAuthUploadState>(createEmptyCodexAuthUpload());
  const [codexAuthUploadErrors, setCodexAuthUploadErrors] =
    useState<CodexAuthUploadErrors>({});
  const [sharingLeaseForm, setSharingLeaseForm] =
    useState<SharingLeaseFormState>(createEmptySharingLeaseForm());
  const [carpoolForm, setCarpoolForm] =
    useState<CarpoolFormState>(createEmptyCarpoolForm());
  const [sharingFormErrors, setSharingFormErrors] =
    useState<SharingFormErrors>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isCodexAuthUploading, setIsCodexAuthUploading] = useState(false);
  const [isSharingSubmitting, setIsSharingSubmitting] = useState(false);
  const [revokingLeaseId, setRevokingLeaseId] = useState<string | null>(null);
  const [removingCarpoolId, setRemovingCarpoolId] = useState<string | null>(
    null,
  );
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

  const { projects, providers, routePolicies, routeReceipts } = result.data;
  const { carpools, leases, sharingUsage } = result.data;
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

  function updateCodexAuthField<K extends keyof CodexAuthUploadState>(
    key: K,
    value: CodexAuthUploadState[K],
  ) {
    setCodexAuthUpload((current) => ({
      ...current,
      [key]: value,
    }));
    setCodexAuthUploadErrors((current) => ({
      ...current,
      [key]: undefined,
    }));
  }

  function updateSharingLeaseField<K extends keyof SharingLeaseFormState>(
    key: K,
    value: SharingLeaseFormState[K],
  ) {
    setSharingLeaseForm((current) => ({
      ...current,
      [key]: value,
    }));
    setSharingFormErrors((current) => ({
      ...current,
      [key]: undefined,
    }));
  }

  function updateCarpoolField<K extends keyof CarpoolFormState>(
    key: K,
    value: CarpoolFormState[K],
  ) {
    setCarpoolForm((current) => ({
      ...current,
      [key]: value,
    }));
    setSharingFormErrors((current) => ({
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
      void refreshRoute();
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

  async function onUploadCodexAuth() {
    const errors: CodexAuthUploadErrors = {};
    if (!codexAuthUpload.displayName.trim()) {
      errors.displayName = "Enter a display name.";
    }
    if (!codexAuthUpload.file) {
      errors.file = "Choose a Codex auth.json file.";
    }
    if (
      codexAuthUpload.providerResourceId.trim() &&
      !/^prvrsrc_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(
        codexAuthUpload.providerResourceId.trim(),
      )
    ) {
      errors.providerResourceId = "Provider resource ids must start with prvrsrc_.";
    }
    if (!/^https:\/\/.+/.test(codexAuthUpload.endpointBaseUrl.trim())) {
      errors.endpointBaseUrl = "Use an HTTPS reverse proxy endpoint.";
    }
    if (!codexAuthUpload.region.trim()) {
      errors.region = "Enter a region label.";
    }
    setCodexAuthUploadErrors(errors);
    if (Object.keys(errors).length > 0 || !codexAuthUpload.file) {
      return;
    }

    setIsCodexAuthUploading(true);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      const authJson = JSON.parse(await codexAuthUpload.file.text());
      const created = await getConsoleDataService().uploadCodexAuthAccount({
        authJson,
        displayName: codexAuthUpload.displayName.trim(),
        endpointBaseUrl: codexAuthUpload.endpointBaseUrl.trim(),
        projectId: codexAuthUpload.projectId || undefined,
        providerResourceId: codexAuthUpload.providerResourceId.trim() || undefined,
        region: codexAuthUpload.region.trim(),
      });
      setCodexAuthUpload(createEmptyCodexAuthUpload());
      setStatusSuccess(
        `Added ${created.displayName} to the Codex auth account pool.`,
      );
      await refreshRoute();
    } catch (error) {
      setStatusError(
        error instanceof SyntaxError
          ? "The selected file is not valid JSON."
          : getControlPlaneActionErrorMessage(error, "codex-auth-upload"),
      );
    } finally {
      setIsCodexAuthUploading(false);
    }
  }

  async function onCreateSharingLease() {
    const errors: SharingFormErrors = {};
    if (!sharingLeaseForm.leaseId.trim()) {
      errors.leaseId = "Enter a lease id.";
    }
    if (!sharingLeaseForm.borrowerWorkspaceId.trim()) {
      errors.borrowerWorkspaceId = "Enter a borrower workspace id.";
    }
    if (!sharingLeaseForm.poolId.trim()) {
      errors.poolId = "Enter a pool id.";
    }
    if (!sharingLeaseForm.expiresAt.trim()) {
      errors.expiresAt = "Enter an expiration timestamp.";
    }
    setSharingFormErrors(errors);
    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSharingSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      await getConsoleDataService().upsertOAuthSharingLease({
        allowedAccountIds: [],
        borrowerWorkspaceId: sharingLeaseForm.borrowerWorkspaceId.trim(),
        expiresAt: sharingLeaseForm.expiresAt.trim(),
        leaseId: sharingLeaseForm.leaseId.trim(),
        maxConcurrentRuns: sharingLeaseForm.maxConcurrentRuns,
        policy: sharingLeaseForm.policy,
        poolId: sharingLeaseForm.poolId.trim(),
        provider: sharingLeaseForm.provider,
        startsAt: new Date().toISOString(),
        status: sharingLeaseForm.status,
        turnBudget: sharingLeaseForm.turnBudget,
      });
      setSharingLeaseForm(createEmptySharingLeaseForm());
      setStatusSuccess("Created sharing lease.");
      await refreshRoute();
    } catch (error) {
      setStatusError(getControlPlaneActionErrorMessage(error, "sharing"));
    } finally {
      setIsSharingSubmitting(false);
    }
  }

  async function onRevokeSharingLease(lease: OAuthSharingLeaseView) {
    setRevokingLeaseId(lease.leaseId);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      await getConsoleDataService().revokeOAuthSharingLease(lease.leaseId);
      setStatusSuccess(`Revoked sharing lease ${lease.leaseId}.`);
      await refreshRoute();
    } catch (error) {
      setStatusError(getControlPlaneActionErrorMessage(error, "sharing"));
    } finally {
      setRevokingLeaseId(null);
    }
  }

  async function onCreateCarpool() {
    const errors: SharingFormErrors = {};
    if (!carpoolForm.carpoolId.trim()) {
      errors.carpoolId = "Enter a carpool id.";
    }
    if (!carpoolForm.name.trim()) {
      errors.name = "Enter a carpool name.";
    }
    if (splitCsv(carpoolForm.memberWorkspaceIds).length === 0) {
      errors.memberWorkspaceIds = "Enter at least one member workspace.";
    }
    if (splitCsv(carpoolForm.poolIds).length === 0) {
      errors.poolIds = "Enter at least one pool id.";
    }
    setSharingFormErrors(errors);
    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSharingSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      await getConsoleDataService().upsertOAuthCarpool({
        carpoolId: carpoolForm.carpoolId.trim(),
        enabled: carpoolForm.enabled,
        memberWorkspaceIds: splitCsv(carpoolForm.memberWorkspaceIds),
        name: carpoolForm.name.trim(),
        perMemberConcurrencyLimit: carpoolForm.perMemberConcurrencyLimit,
        perMemberTurnBudget: carpoolForm.perMemberTurnBudget,
        poolIds: splitCsv(carpoolForm.poolIds),
        provider: carpoolForm.provider,
        strategy: carpoolForm.strategy,
      });
      setCarpoolForm(createEmptyCarpoolForm());
      setStatusSuccess("Created carpool.");
      await refreshRoute();
    } catch (error) {
      setStatusError(getControlPlaneActionErrorMessage(error, "sharing"));
    } finally {
      setIsSharingSubmitting(false);
    }
  }

  async function onRemoveCarpool(carpool: OAuthCarpoolView) {
    setRemovingCarpoolId(carpool.carpoolId);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      await getConsoleDataService().removeOAuthCarpool(carpool.carpoolId);
      setStatusSuccess(`Disabled carpool ${carpool.name}.`);
      await refreshRoute();
    } catch (error) {
      setStatusError(getControlPlaneActionErrorMessage(error, "sharing"));
    } finally {
      setRemovingCarpoolId(null);
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
          <Text fw={700}>Codex auth account pool</Text>
          <Badge color="grape" variant="light">
            encrypted upload
          </Badge>
        </Group>
        <Stack>
          <Group grow>
            <TextInput
              label="Account display name"
              onChange={(event) =>
                updateCodexAuthField("displayName", event.currentTarget.value)
              }
              placeholder="Team Codex account"
              value={codexAuthUpload.displayName}
            />
            <Select
              data={projectOptions}
              label="Project scope"
              onChange={(value) =>
                updateCodexAuthField("projectId", value ?? "")
              }
              value={codexAuthUpload.projectId}
            />
          </Group>
          <Group grow>
            <FieldErrorText error={codexAuthUploadErrors.displayName} />
            <FieldErrorText error={codexAuthUploadErrors.projectId} />
          </Group>
          <Group grow>
            <TextInput
              label="Pool provider resource id"
              onChange={(event) =>
                updateCodexAuthField(
                  "providerResourceId",
                  event.currentTarget.value,
                )
              }
              placeholder="Leave blank to create one"
              value={codexAuthUpload.providerResourceId}
            />
            <TextInput
              label="Pool region"
              onChange={(event) =>
                updateCodexAuthField("region", event.currentTarget.value)
              }
              placeholder="global"
              value={codexAuthUpload.region}
            />
          </Group>
          <Group grow>
            <FieldErrorText error={codexAuthUploadErrors.providerResourceId} />
            <FieldErrorText error={codexAuthUploadErrors.region} />
          </Group>
          <TextInput
            label="Reverse proxy endpoint"
            onChange={(event) =>
              updateCodexAuthField("endpointBaseUrl", event.currentTarget.value)
            }
            placeholder="https://chatgpt-reverse-proxy.example.com/v1"
            value={codexAuthUpload.endpointBaseUrl}
          />
          <FieldErrorText error={codexAuthUploadErrors.endpointBaseUrl} />
          <FileInput
            accept="application/json,.json"
            clearable
            label="Codex auth.json"
            onChange={(file) => updateCodexAuthField("file", file)}
            placeholder="Choose auth.json"
            value={codexAuthUpload.file}
          />
          <FieldErrorText error={codexAuthUploadErrors.file} />
          <Group justify="flex-end">
            <Button
              loading={isCodexAuthUploading}
              onClick={() => void onUploadCodexAuth()}
            >
              Add to pool
            </Button>
          </Group>
        </Stack>
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Pools sharing</Text>
          <Badge color="teal" variant="light">
            runtime-owned
          </Badge>
        </Group>
        <Stack>
          <Group align="flex-end" grow>
            <TextInput
              label="Lease id"
              onChange={(event) =>
                updateSharingLeaseField("leaseId", event.currentTarget.value)
              }
              placeholder="lease_codex_acme_support"
              value={sharingLeaseForm.leaseId}
            />
            <TextInput
              label="Borrower workspace"
              onChange={(event) =>
                updateSharingLeaseField(
                  "borrowerWorkspaceId",
                  event.currentTarget.value,
                )
              }
              value={sharingLeaseForm.borrowerWorkspaceId}
            />
            <TextInput
              label="Pool id"
              onChange={(event) =>
                updateSharingLeaseField("poolId", event.currentTarget.value)
              }
              placeholder="prvrsrc_codex_team"
              value={sharingLeaseForm.poolId}
            />
          </Group>
          <Group grow>
            <FieldErrorText error={sharingFormErrors.leaseId} />
            <FieldErrorText error={sharingFormErrors.borrowerWorkspaceId} />
            <FieldErrorText error={sharingFormErrors.poolId} />
          </Group>
          <Group align="flex-end" grow>
            <Select
              data={[
                { label: "codex", value: "codex" },
                { label: "gemini", value: "gemini" },
                { label: "claude_code", value: "claude_code" },
              ]}
              label="Provider"
              onChange={(value) =>
                updateSharingLeaseField(
                  "provider",
                  (value ?? "codex") as SharingLeaseFormState["provider"],
                )
              }
              value={sharingLeaseForm.provider}
            />
            <Select
              data={[
                { label: "active", value: "active" },
                { label: "paused", value: "paused" },
                { label: "pending", value: "pending" },
              ]}
              label="Lease status"
              onChange={(value) =>
                updateSharingLeaseField(
                  "status",
                  (value ?? "active") as SharingLeaseFormState["status"],
                )
              }
              value={sharingLeaseForm.status}
            />
            <Select
              data={[
                { label: "fair_share", value: "fair_share" },
                { label: "owner_priority", value: "owner_priority" },
                { label: "borrower_priority", value: "borrower_priority" },
              ]}
              label="Policy"
              onChange={(value) =>
                updateSharingLeaseField(
                  "policy",
                  (value ?? "fair_share") as SharingLeaseFormState["policy"],
                )
              }
              value={sharingLeaseForm.policy}
            />
          </Group>
          <Group align="flex-end" grow>
            <NumberInput
              label="Max concurrent runs"
              min={1}
              onChange={(value) =>
                updateSharingLeaseField(
                  "maxConcurrentRuns",
                  Number(value) || 1,
                )
              }
              value={sharingLeaseForm.maxConcurrentRuns}
            />
            <NumberInput
              label="Turn budget"
              min={1}
              onChange={(value) =>
                updateSharingLeaseField("turnBudget", Number(value) || 1)
              }
              value={sharingLeaseForm.turnBudget}
            />
            <TextInput
              label="Expires at"
              onChange={(event) =>
                updateSharingLeaseField("expiresAt", event.currentTarget.value)
              }
              value={sharingLeaseForm.expiresAt}
            />
          </Group>
          <FieldErrorText error={sharingFormErrors.expiresAt} />
          <Group justify="flex-end">
            <Button
              loading={isSharingSubmitting}
              onClick={() => void onCreateSharingLease()}
            >
              Create lease
            </Button>
          </Group>
          {leases.length === 0 ? (
            <EmptyCollectionState
              description="Create a sharing lease to authorize a borrower workspace for a pool."
              title="No sharing leases"
            />
          ) : (
            <Table striped withTableBorder>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Lease</Table.Th>
                  <Table.Th>Borrower</Table.Th>
                  <Table.Th>Pool</Table.Th>
                  <Table.Th>Budget</Table.Th>
                  <Table.Th>Status</Table.Th>
                  <Table.Th>Actions</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {leases.map((lease) => (
                  <Table.Tr key={lease.leaseId}>
                    <Table.Td>{lease.leaseId}</Table.Td>
                    <Table.Td>{lease.borrowerWorkspaceId}</Table.Td>
                    <Table.Td>{lease.poolId}</Table.Td>
                    <Table.Td>
                      {lease.turnBudget ?? "unlimited"} turns /{" "}
                      {lease.maxConcurrentRuns} concurrent
                    </Table.Td>
                    <Table.Td>
                      <Badge variant="light">{lease.status}</Badge>
                    </Table.Td>
                    <Table.Td>
                      <Button
                        disabled={lease.status === "revoked"}
                        loading={revokingLeaseId === lease.leaseId}
                        onClick={() => void onRevokeSharingLease(lease)}
                        size="xs"
                        variant="subtle"
                      >
                        Revoke
                      </Button>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          )}
          <Group align="flex-end" grow>
            <TextInput
              label="Carpool id"
              onChange={(event) =>
                updateCarpoolField("carpoolId", event.currentTarget.value)
              }
              placeholder="carpool_codex_acme"
              value={carpoolForm.carpoolId}
            />
            <TextInput
              label="Name"
              onChange={(event) =>
                updateCarpoolField("name", event.currentTarget.value)
              }
              value={carpoolForm.name}
            />
            <TextInput
              label="Pool ids"
              onChange={(event) =>
                updateCarpoolField("poolIds", event.currentTarget.value)
              }
              placeholder="prvrsrc_codex_team"
              value={carpoolForm.poolIds}
            />
          </Group>
          <Group grow>
            <FieldErrorText error={sharingFormErrors.carpoolId} />
            <FieldErrorText error={sharingFormErrors.name} />
            <FieldErrorText error={sharingFormErrors.poolIds} />
          </Group>
          <Group align="flex-end" grow>
            <TextInput
              label="Member workspaces"
              onChange={(event) =>
                updateCarpoolField(
                  "memberWorkspaceIds",
                  event.currentTarget.value,
                )
              }
              value={carpoolForm.memberWorkspaceIds}
            />
            <Select
              data={[
                { label: "fair_share", value: "fair_share" },
                { label: "weighted", value: "weighted" },
                { label: "cheapest_ready", value: "cheapest_ready" },
                { label: "fastest_ready", value: "fastest_ready" },
              ]}
              label="Strategy"
              onChange={(value) =>
                updateCarpoolField(
                  "strategy",
                  (value ?? "fair_share") as CarpoolFormState["strategy"],
                )
              }
              value={carpoolForm.strategy}
            />
            <Switch
              checked={carpoolForm.enabled}
              label="Enabled"
              onChange={(event) =>
                updateCarpoolField("enabled", event.currentTarget.checked)
              }
            />
          </Group>
          <FieldErrorText error={sharingFormErrors.memberWorkspaceIds} />
          <Group align="flex-end" grow>
            <NumberInput
              label="Per-member concurrency"
              min={1}
              onChange={(value) =>
                updateCarpoolField(
                  "perMemberConcurrencyLimit",
                  Number(value) || 1,
                )
              }
              value={carpoolForm.perMemberConcurrencyLimit}
            />
            <NumberInput
              label="Per-member turns"
              min={1}
              onChange={(value) =>
                updateCarpoolField("perMemberTurnBudget", Number(value) || 1)
              }
              value={carpoolForm.perMemberTurnBudget}
            />
            <Button
              loading={isSharingSubmitting}
              onClick={() => void onCreateCarpool()}
            >
              Create carpool
            </Button>
          </Group>
          {carpools.length === 0 ? (
            <EmptyCollectionState
              description="Create a carpool to share one or more pools across member workspaces."
              title="No carpools"
            />
          ) : (
            <Table striped withTableBorder>
              <Table.Thead>
                <Table.Tr>
                  <Table.Th>Carpool</Table.Th>
                  <Table.Th>Members</Table.Th>
                  <Table.Th>Pools</Table.Th>
                  <Table.Th>Limits</Table.Th>
                  <Table.Th>Status</Table.Th>
                  <Table.Th>Actions</Table.Th>
                </Table.Tr>
              </Table.Thead>
              <Table.Tbody>
                {carpools.map((carpool) => (
                  <Table.Tr key={carpool.carpoolId}>
                    <Table.Td>{carpool.name}</Table.Td>
                    <Table.Td>{carpool.memberWorkspaceIds.join(", ")}</Table.Td>
                    <Table.Td>{carpool.poolIds.join(", ")}</Table.Td>
                    <Table.Td>
                      {carpool.perMemberTurnBudget ?? "unlimited"} turns /{" "}
                      {carpool.perMemberConcurrencyLimit ?? "unlimited"} concurrent
                    </Table.Td>
                    <Table.Td>
                      <Badge color={carpool.enabled ? "teal" : "gray"} variant="light">
                        {carpool.enabled ? "enabled" : "disabled"}
                      </Badge>
                    </Table.Td>
                    <Table.Td>
                      <Button
                        loading={removingCarpoolId === carpool.carpoolId}
                        onClick={() => void onRemoveCarpool(carpool)}
                        size="xs"
                        variant="subtle"
                      >
                        Disable
                      </Button>
                    </Table.Td>
                  </Table.Tr>
                ))}
              </Table.Tbody>
            </Table>
          )}
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Usage scope</Table.Th>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Account</Table.Th>
                <Table.Th>Turns</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {sharingUsage.rows.slice(0, 6).map((row, index) => (
                <Table.Tr key={`${row.leaseId ?? row.carpoolId ?? "usage"}-${index}`}>
                  <Table.Td>
                    {row.leaseId ?? row.carpoolId ?? row.workspaceId ?? "pool"}
                  </Table.Td>
                  <Table.Td>{row.provider}</Table.Td>
                  <Table.Td>{row.accountId ?? "all accounts"}</Table.Td>
                  <Table.Td>{row.turns}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        </Stack>
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
                <Table.Th>Protocols</Table.Th>
                <Table.Th>Capabilities</Table.Th>
                <Table.Th>Health</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Latest route signal</Table.Th>
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
                    <Text size="sm">
                      {provider.supported_protocol_families.join(", ") || "None"}
                    </Text>
                  </Table.Td>
                  <Table.Td>
                    <Text size="sm">
                      {providerCapabilityLabels(provider).join(", ") || "None"}
                    </Text>
                  </Table.Td>
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
                  <Table.Td>
                    <Text c="dimmed" size="sm">
                      {latestSignalForProvider(
                        provider,
                        routeReceipts,
                        routePolicyNames,
                      ) ?? "No recent receipt"}
                    </Text>
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
        (target) => target.provider_resource_id === provider.provider_resource_id,
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
