import {
  UiChip,
  UiButton,
  UiSurface,
  UiCheckbox,
  UiFileField,
  UiInline,
  UiNumberField,
  UiSelect,
  UiStack,
  UiSwitch,
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
  type OAuthCarpoolView,
  type OAuthSharingLeaseView,
  type OAuthSharingUsageView,
} from "../features/control-plane/service";
import {
  authKindOptions,
  buildCarpoolInput,
  buildCodexAuthUploadInput,
  buildProviderMutationInput,
  buildSharingLeaseInput,
  createEmptyCarpoolForm,
  createEmptyCodexAuthUpload,
  createEmptyProviderForm,
  createEmptySharingLeaseForm,
  credentialOwnerOptions,
  deploymentScopeOptions,
  latestSignalForProvider,
  providerCapabilityLabels,
  providerHealthOptions,
  providerStatusOptions,
  providerToFormState,
  provenanceOptions,
  validateCarpoolForm,
  validateCodexAuthUpload,
  validateProviderForm,
  validateSharingLeaseForm,
  type CarpoolFormState,
  type CodexAuthUploadErrors,
  type CodexAuthUploadState,
  type ProviderFormErrors,
  type ProviderFormState,
  type SharingFormErrors,
  type SharingLeaseFormState,
} from "../features/control-plane/providers-view-model";
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
  const [editingProvider, setEditingProvider] =
    useState<ProviderResource | null>(null);
  const [formErrors, setFormErrors] = useState<ProviderFormErrors>({});
  const [formMode, setFormMode] = useState<"create" | "edit" | null>(null);
  const [formState, setFormState] = useState<ProviderFormState>(
    createEmptyProviderForm(),
  );
  const [codexAuthUpload, setCodexAuthUpload] = useState<CodexAuthUploadState>(
    createEmptyCodexAuthUpload(),
  );
  const [codexAuthUploadErrors, setCodexAuthUploadErrors] =
    useState<CodexAuthUploadErrors>({});
  const [sharingLeaseForm, setSharingLeaseForm] =
    useState<SharingLeaseFormState>(createEmptySharingLeaseForm());
  const [carpoolForm, setCarpoolForm] = useState<CarpoolFormState>(
    createEmptyCarpoolForm(),
  );
  const [sharingFormErrors, setSharingFormErrors] = useState<SharingFormErrors>(
    {},
  );
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

    const input = buildProviderMutationInput(formState, editingProvider);

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
    const errors = validateCodexAuthUpload(codexAuthUpload);
    setCodexAuthUploadErrors(errors);
    if (Object.keys(errors).length > 0 || !codexAuthUpload.file) {
      return;
    }

    setIsCodexAuthUploading(true);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      const authJson = JSON.parse(await codexAuthUpload.file.text());
      const created = await getConsoleDataService().uploadCodexAuthAccount(
        buildCodexAuthUploadInput(codexAuthUpload, authJson),
      );
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
    const errors = validateSharingLeaseForm(sharingLeaseForm);
    setSharingFormErrors(errors);
    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSharingSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      await getConsoleDataService().upsertOAuthSharingLease(
        buildSharingLeaseInput(sharingLeaseForm),
      );
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
    const errors = validateCarpoolForm(carpoolForm);
    setSharingFormErrors(errors);
    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSharingSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);
    try {
      await getConsoleDataService().upsertOAuthCarpool(
        buildCarpoolInput(carpoolForm),
      );
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
          <UiText fw={700}>Codex auth account pool</UiText>
          <UiChip color="grape" variant="light">
            encrypted upload
          </UiChip>
        </UiInline>
        <UiStack>
          <UiInline grow>
            <UiTextField
              label="Account display name"
              onChange={(event) =>
                updateCodexAuthField("displayName", event.currentTarget.value)
              }
              placeholder="Team Codex account"
              value={codexAuthUpload.displayName}
            />
            <UiSelect
              data={projectOptions}
              label="Project scope"
              onChange={(value) =>
                updateCodexAuthField("projectId", value ?? "")
              }
              value={codexAuthUpload.projectId}
            />
          </UiInline>
          <UiInline grow>
            <FieldErrorText error={codexAuthUploadErrors.displayName} />
            <FieldErrorText error={codexAuthUploadErrors.projectId} />
          </UiInline>
          <UiInline grow>
            <UiTextField
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
            <UiTextField
              label="Pool region"
              onChange={(event) =>
                updateCodexAuthField("region", event.currentTarget.value)
              }
              placeholder="global"
              value={codexAuthUpload.region}
            />
          </UiInline>
          <UiInline grow>
            <FieldErrorText error={codexAuthUploadErrors.providerResourceId} />
            <FieldErrorText error={codexAuthUploadErrors.region} />
          </UiInline>
          <UiTextField
            label="Reverse proxy endpoint"
            onChange={(event) =>
              updateCodexAuthField("endpointBaseUrl", event.currentTarget.value)
            }
            placeholder="https://chatgpt-reverse-proxy.example.com/v1"
            value={codexAuthUpload.endpointBaseUrl}
          />
          <FieldErrorText error={codexAuthUploadErrors.endpointBaseUrl} />
          <UiFileField
            accept="application/json,.json"
            clearable
            label="Codex auth.json"
            onChange={(file) => updateCodexAuthField("file", file)}
            placeholder="Choose auth.json"
            value={codexAuthUpload.file}
          />
          <FieldErrorText error={codexAuthUploadErrors.file} />
          <UiInline justify="flex-end">
            <UiButton
              loading={isCodexAuthUploading}
              onClick={() => void onUploadCodexAuth()}
            >
              Add to pool
            </UiButton>
          </UiInline>
        </UiStack>
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Pools sharing</UiText>
          <UiChip color="teal" variant="light">
            runtime-owned
          </UiChip>
        </UiInline>
        <UiStack>
          <UiInline align="flex-end" grow>
            <UiTextField
              label="Lease id"
              onChange={(event) =>
                updateSharingLeaseField("leaseId", event.currentTarget.value)
              }
              placeholder="lease_codex_acme_support"
              value={sharingLeaseForm.leaseId}
            />
            <UiTextField
              label="Borrower workspace"
              onChange={(event) =>
                updateSharingLeaseField(
                  "borrowerWorkspaceId",
                  event.currentTarget.value,
                )
              }
              value={sharingLeaseForm.borrowerWorkspaceId}
            />
            <UiTextField
              label="Pool id"
              onChange={(event) =>
                updateSharingLeaseField("poolId", event.currentTarget.value)
              }
              placeholder="prvrsrc_codex_team"
              value={sharingLeaseForm.poolId}
            />
          </UiInline>
          <UiInline grow>
            <FieldErrorText error={sharingFormErrors.leaseId} />
            <FieldErrorText error={sharingFormErrors.borrowerWorkspaceId} />
            <FieldErrorText error={sharingFormErrors.poolId} />
          </UiInline>
          <UiInline align="flex-end" grow>
            <UiSelect
              data={["codex", "gemini", "claude_code"]}
              label="Provider"
              onChange={(value) =>
                updateSharingLeaseField(
                  "provider",
                  (value ?? "codex") as SharingLeaseFormState["provider"],
                )
              }
              value={sharingLeaseForm.provider}
            />
            <UiSelect
              data={["active", "paused", "pending"]}
              label="Lease status"
              onChange={(value) =>
                updateSharingLeaseField(
                  "status",
                  (value ?? "active") as SharingLeaseFormState["status"],
                )
              }
              value={sharingLeaseForm.status}
            />
            <UiSelect
              data={["fair_share", "owner_priority", "borrower_priority"]}
              label="Policy"
              onChange={(value) =>
                updateSharingLeaseField(
                  "policy",
                  (value ?? "fair_share") as SharingLeaseFormState["policy"],
                )
              }
              value={sharingLeaseForm.policy}
            />
          </UiInline>
          <UiInline align="flex-end" grow>
            <UiNumberField
              label="Max concurrent runs"
              min={1}
              onChange={(value) =>
                updateSharingLeaseField("maxConcurrentRuns", Number(value) || 1)
              }
              value={sharingLeaseForm.maxConcurrentRuns}
            />
            <UiNumberField
              label="Turn budget"
              min={1}
              onChange={(value) =>
                updateSharingLeaseField("turnBudget", Number(value) || 1)
              }
              value={sharingLeaseForm.turnBudget}
            />
            <UiTextField
              label="Expires at"
              onChange={(event) =>
                updateSharingLeaseField("expiresAt", event.currentTarget.value)
              }
              value={sharingLeaseForm.expiresAt}
            />
          </UiInline>
          <FieldErrorText error={sharingFormErrors.expiresAt} />
          <UiInline justify="flex-end">
            <UiButton
              loading={isSharingSubmitting}
              onClick={() => void onCreateSharingLease()}
            >
              Create lease
            </UiButton>
          </UiInline>
          {leases.length === 0 ? (
            <EmptyCollectionState
              description="Create a sharing lease to authorize a borrower workspace for a pool."
              title="No sharing leases"
            />
          ) : (
            <UiDataTable striped withTableBorder>
              <UiDataTable.Thead>
                <UiDataTable.Tr>
                  <UiDataTable.Th>Lease</UiDataTable.Th>
                  <UiDataTable.Th>Borrower</UiDataTable.Th>
                  <UiDataTable.Th>Pool</UiDataTable.Th>
                  <UiDataTable.Th>Budget</UiDataTable.Th>
                  <UiDataTable.Th>Status</UiDataTable.Th>
                  <UiDataTable.Th>Actions</UiDataTable.Th>
                </UiDataTable.Tr>
              </UiDataTable.Thead>
              <UiDataTable.Tbody>
                {leases.map((lease) => (
                  <UiDataTable.Tr key={lease.leaseId}>
                    <UiDataTable.Td>{lease.leaseId}</UiDataTable.Td>
                    <UiDataTable.Td>{lease.borrowerWorkspaceId}</UiDataTable.Td>
                    <UiDataTable.Td>{lease.poolId}</UiDataTable.Td>
                    <UiDataTable.Td>
                      {lease.turnBudget ?? "unlimited"} turns /{" "}
                      {lease.maxConcurrentRuns} concurrent
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiChip variant="light">{lease.status}</UiChip>
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiButton
                        disabled={lease.status === "revoked"}
                        loading={revokingLeaseId === lease.leaseId}
                        onClick={() => void onRevokeSharingLease(lease)}
                        size="xs"
                        variant="subtle"
                      >
                        Revoke
                      </UiButton>
                    </UiDataTable.Td>
                  </UiDataTable.Tr>
                ))}
              </UiDataTable.Tbody>
            </UiDataTable>
          )}
          <UiInline align="flex-end" grow>
            <UiTextField
              label="Carpool id"
              onChange={(event) =>
                updateCarpoolField("carpoolId", event.currentTarget.value)
              }
              placeholder="carpool_codex_acme"
              value={carpoolForm.carpoolId}
            />
            <UiTextField
              label="Name"
              onChange={(event) =>
                updateCarpoolField("name", event.currentTarget.value)
              }
              value={carpoolForm.name}
            />
            <UiTextField
              label="Pool ids"
              onChange={(event) =>
                updateCarpoolField("poolIds", event.currentTarget.value)
              }
              placeholder="prvrsrc_codex_team"
              value={carpoolForm.poolIds}
            />
          </UiInline>
          <UiInline grow>
            <FieldErrorText error={sharingFormErrors.carpoolId} />
            <FieldErrorText error={sharingFormErrors.name} />
            <FieldErrorText error={sharingFormErrors.poolIds} />
          </UiInline>
          <UiInline align="flex-end" grow>
            <UiTextField
              label="Member workspaces"
              onChange={(event) =>
                updateCarpoolField(
                  "memberWorkspaceIds",
                  event.currentTarget.value,
                )
              }
              value={carpoolForm.memberWorkspaceIds}
            />
            <UiSelect
              data={[
                "fair_share",
                "weighted",
                "cheapest_ready",
                "fastest_ready",
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
            <UiSwitch
              checked={carpoolForm.enabled}
              label="Enabled"
              onChange={(event) =>
                updateCarpoolField("enabled", event.currentTarget.checked)
              }
            />
          </UiInline>
          <FieldErrorText error={sharingFormErrors.memberWorkspaceIds} />
          <UiInline align="flex-end" grow>
            <UiNumberField
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
            <UiNumberField
              label="Per-member turns"
              min={1}
              onChange={(value) =>
                updateCarpoolField("perMemberTurnBudget", Number(value) || 1)
              }
              value={carpoolForm.perMemberTurnBudget}
            />
            <UiButton
              loading={isSharingSubmitting}
              onClick={() => void onCreateCarpool()}
            >
              Create carpool
            </UiButton>
          </UiInline>
          {carpools.length === 0 ? (
            <EmptyCollectionState
              description="Create a carpool to share one or more pools across member workspaces."
              title="No carpools"
            />
          ) : (
            <UiDataTable striped withTableBorder>
              <UiDataTable.Thead>
                <UiDataTable.Tr>
                  <UiDataTable.Th>Carpool</UiDataTable.Th>
                  <UiDataTable.Th>Members</UiDataTable.Th>
                  <UiDataTable.Th>Pools</UiDataTable.Th>
                  <UiDataTable.Th>Limits</UiDataTable.Th>
                  <UiDataTable.Th>Status</UiDataTable.Th>
                  <UiDataTable.Th>Actions</UiDataTable.Th>
                </UiDataTable.Tr>
              </UiDataTable.Thead>
              <UiDataTable.Tbody>
                {carpools.map((carpool) => (
                  <UiDataTable.Tr key={carpool.carpoolId}>
                    <UiDataTable.Td>{carpool.name}</UiDataTable.Td>
                    <UiDataTable.Td>
                      {carpool.memberWorkspaceIds.join(", ")}
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      {carpool.poolIds.join(", ")}
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      {carpool.perMemberTurnBudget ?? "unlimited"} turns /{" "}
                      {carpool.perMemberConcurrencyLimit ?? "unlimited"}{" "}
                      concurrent
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiChip
                        color={carpool.enabled ? "teal" : "gray"}
                        variant="light"
                      >
                        {carpool.enabled ? "enabled" : "disabled"}
                      </UiChip>
                    </UiDataTable.Td>
                    <UiDataTable.Td>
                      <UiButton
                        loading={removingCarpoolId === carpool.carpoolId}
                        onClick={() => void onRemoveCarpool(carpool)}
                        size="xs"
                        variant="subtle"
                      >
                        Disable
                      </UiButton>
                    </UiDataTable.Td>
                  </UiDataTable.Tr>
                ))}
              </UiDataTable.Tbody>
            </UiDataTable>
          )}
          <UiDataTable striped withTableBorder>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Usage scope</UiDataTable.Th>
                <UiDataTable.Th>Provider</UiDataTable.Th>
                <UiDataTable.Th>Account</UiDataTable.Th>
                <UiDataTable.Th>Turns</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {sharingUsage.rows.slice(0, 6).map((row, index) => (
                <UiDataTable.Tr
                  key={(row.leaseId ?? row.carpoolId ?? "usage") + "-" + index}
                >
                  <UiDataTable.Td>
                    {row.leaseId ?? row.carpoolId ?? row.workspaceId ?? "pool"}
                  </UiDataTable.Td>
                  <UiDataTable.Td>{row.provider}</UiDataTable.Td>
                  <UiDataTable.Td>
                    {row.accountId ?? "all accounts"}
                  </UiDataTable.Td>
                  <UiDataTable.Td>{row.turns}</UiDataTable.Td>
                </UiDataTable.Tr>
              ))}
            </UiDataTable.Tbody>
          </UiDataTable>
        </UiStack>
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
