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
  type ConfigSnapshotMutationInput,
} from "../features/control-plane/service";
import type {
  ConfigSnapshotView,
  ProjectSummary,
  RoutePolicyView,
} from "../features/control-plane/types";
import {
  ActionStatusNotice,
  FieldErrorText,
} from "../features/control-plane/workflow-ui";

type SnapshotsPageData = {
  projects: ProjectSummary[];
  providers: ProviderResource[];
  routePolicies: RoutePolicyView[];
  snapshots: ConfigSnapshotView[];
};

type SnapshotFormState = {
  budgetPolicyId: string;
  configSnapshotId: string;
  projectId: string;
  providerResourceIds: string[];
  revision: string;
  routePolicyId: string;
};

type SnapshotFormErrors = Partial<Record<keyof SnapshotFormState, string>>;

function createEmptySnapshotForm(): SnapshotFormState {
  return {
    budgetPolicyId: "budgetpol_default",
    configSnapshotId: "",
    projectId: "",
    providerResourceIds: [],
    revision: "1",
    routePolicyId: "",
  };
}

function validateSnapshotForm(form: SnapshotFormState) {
  const errors: SnapshotFormErrors = {};

  if (!/^cfgsnap_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.configSnapshotId)) {
    errors.configSnapshotId =
      "Use an id that starts with cfgsnap_ and contains letters, digits, _ or -.";
  }

  if (!form.projectId) {
    errors.projectId = "Select a project.";
  }

  if (!form.routePolicyId) {
    errors.routePolicyId = "Select a route policy.";
  }

  if (form.providerResourceIds.length === 0) {
    errors.providerResourceIds = "Select at least one provider resource.";
  }

  const revision = Number(form.revision);
  if (!Number.isInteger(revision) || revision < 1) {
    errors.revision = "Enter a positive revision number.";
  }

  if (
    !/^budgetpol_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.budgetPolicyId.trim())
  ) {
    errors.budgetPolicyId = "Budget policy ids must start with budgetpol_.";
  }

  return errors;
}

export const Route = createFileRoute("/app/snapshots")({
  loader: () =>
    loadRouteData(async () => {
      const [snapshots, routePolicies, providers, projects] = await Promise.all(
        [
          getConsoleDataService().listConfigSnapshots(),
          getConsoleDataService().listRoutePolicies(),
          getConsoleDataService().listProviderResources(),
          getConsoleDataService().listProjects(),
        ],
      );

      return {
        projects,
        providers,
        routePolicies,
        snapshots,
      } satisfies SnapshotsPageData;
    }),
  pendingComponent: () => <RouteLoadingState label="Loading snapshots" />,
  pendingMs: 0,
  component: ConfigSnapshotsPage,
});

function ConfigSnapshotsPage() {
  const result = Route.useLoaderData();
  const router = useRouter();
  const [activatingSnapshotId, setActivatingSnapshotId] = useState<
    string | null
  >(null);
  const [formErrors, setFormErrors] = useState<SnapshotFormErrors>({});
  const [formOpen, setFormOpen] = useState(false);
  const [formState, setFormState] = useState<SnapshotFormState>(
    createEmptySnapshotForm(),
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [lastActivatedSnapshotId, setLastActivatedSnapshotId] = useState<
    string | null
  >(null);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusSuccess, setStatusSuccess] = useState<string | null>(null);

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Review config snapshots by tenant, and activate one snapshot as the live control-plane source."
          title="Snapshots"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="Snapshot catalog could not be loaded from the control-plane service."
          title="Snapshots unavailable"
        />
      </UiStack>
    );
  }

  const { projects, providers, routePolicies, snapshots } = result.data;

  function openCreateForm() {
    setFormErrors({});
    setFormOpen(true);
    setFormState({
      ...createEmptySnapshotForm(),
      projectId: projects[0]?.id ?? "",
      routePolicyId: routePolicies[0]?.id ?? "",
    });
  }

  function updateField<K extends keyof SnapshotFormState>(
    key: K,
    value: SnapshotFormState[K],
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
    setFormState(createEmptySnapshotForm());
  }

  async function onCreateSnapshot() {
    const errors = validateSnapshotForm(formState);
    setFormErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);

    const input: ConfigSnapshotMutationInput = {
      budgetPolicyId: formState.budgetPolicyId.trim(),
      configSnapshotId: formState.configSnapshotId.trim(),
      projectId: formState.projectId,
      providerResourceIds: formState.providerResourceIds,
      revision: Number(formState.revision),
      routePolicyId: formState.routePolicyId,
      status: "draft",
    };

    try {
      await getConsoleDataService().createConfigSnapshot(input);
      setStatusSuccess(
        `Created config snapshot ${formState.configSnapshotId}.`,
      );
      resetForm();
      await router.invalidate();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "snapshot-create"),
      );
    } finally {
      setIsSubmitting(false);
    }
  }

  async function onActivate(configSnapshotId: string) {
    setActivatingSnapshotId(configSnapshotId);
    setLastActivatedSnapshotId(null);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      await getConsoleDataService().activateConfigSnapshot(configSnapshotId);
      setLastActivatedSnapshotId(configSnapshotId);
      setStatusSuccess(`Activated snapshot ${configSnapshotId}.`);
      await router.invalidate();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "snapshot-activate"),
      );
    } finally {
      setActivatingSnapshotId(null);
    }
  }

  return (
    <UiStack>
      <PageHeader
        description="Review config snapshots by tenant, and activate one snapshot as the live control-plane source."
        title="Snapshots"
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
          <UiText fw={700}>Create config snapshot</UiText>
          <UiInline>
            {formOpen ? (
              <UiButton onClick={resetForm} size="sm" variant="subtle">
                Cancel
              </UiButton>
            ) : null}
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
              {formOpen ? "Hide form" : "Create snapshot"}
            </UiButton>
          </UiInline>
        </UiInline>
        {formOpen ? (
          <UiStack>
            <UiTextField
              label="Snapshot id"
              onChange={(event) =>
                updateField("configSnapshotId", event.currentTarget.value)
              }
              placeholder="cfgsnap_gateway_v3"
              value={formState.configSnapshotId}
            />
            <FieldErrorText error={formErrors.configSnapshotId} />
            <UiInline grow>
              <UiSelect
                data={projects.map((project) => ({
                  label: project.name,
                  value: project.id,
                }))}
                label="Project"
                onChange={(value) => updateField("projectId", value ?? "")}
                value={formState.projectId}
              />
              <UiTextField
                label="Revision"
                onChange={(event) =>
                  updateField("revision", event.currentTarget.value)
                }
                value={formState.revision}
              />
            </UiInline>
            <UiInline grow>
              <FieldErrorText error={formErrors.projectId} />
              <FieldErrorText error={formErrors.revision} />
            </UiInline>
            <UiSelect
              data={routePolicies.map((policy) => ({
                label: `${policy.name} (${policy.modelAlias})`,
                value: policy.id,
              }))}
              label="Route policy"
              onChange={(value) => updateField("routePolicyId", value ?? "")}
              value={formState.routePolicyId}
            />
            <FieldErrorText error={formErrors.routePolicyId} />
            <UiTextField
              label="Budget policy id"
              onChange={(event) =>
                updateField("budgetPolicyId", event.currentTarget.value)
              }
              value={formState.budgetPolicyId}
            />
            <FieldErrorText error={formErrors.budgetPolicyId} />
            <UiStack gap="xs">
              <UiText fw={600} size="sm">
                Provider assignments
              </UiText>
              {providers.map((provider) => {
                const checked = formState.providerResourceIds.includes(
                  provider.provider_resource_id,
                );

                return (
                  <UiCheckbox
                    checked={checked}
                    key={provider.provider_resource_id}
                    label={`${provider.name} (${provider.provider_resource_id})`}
                    onChange={(event) => {
                      const nextIds = event.currentTarget.checked
                        ? [
                            ...formState.providerResourceIds,
                            provider.provider_resource_id,
                          ]
                        : formState.providerResourceIds.filter(
                            (providerId) =>
                              providerId !== provider.provider_resource_id,
                          );
                      updateField("providerResourceIds", nextIds);
                    }}
                  />
                );
              })}
            </UiStack>
            <FieldErrorText error={formErrors.providerResourceIds} />
            <UiInline justify="flex-end">
              <UiButton
                loading={isSubmitting}
                onClick={() => void onCreateSnapshot()}
              >
                Save snapshot
              </UiButton>
            </UiInline>
          </UiStack>
        ) : (
          <UiText c="dimmed" size="sm">
            Draft a new config snapshot from the current provider and route
            policy inventory.
          </UiText>
        )}
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Config snapshots</UiText>
          <UiChip color="blue" variant="light">
            {snapshots.length} items
          </UiChip>
        </UiInline>
        {snapshots.length === 0 ? (
          <EmptyCollectionState
            description="No snapshots have been created yet."
            title="No snapshots"
          />
        ) : (
          <UiDataTable striped withTableBorder>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Snapshot ID</UiDataTable.Th>
                <UiDataTable.Th>Project</UiDataTable.Th>
                <UiDataTable.Th>Revision</UiDataTable.Th>
                <UiDataTable.Th>Status</UiDataTable.Th>
                <UiDataTable.Th>Route policy</UiDataTable.Th>
                <UiDataTable.Th>Providers</UiDataTable.Th>
                <UiDataTable.Th>Activated at</UiDataTable.Th>
                <UiDataTable.Th>Actions</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {snapshots.map((snapshot) => (
                <UiDataTable.Tr key={snapshot.configSnapshotId}>
                  {(() => {
                    const isActive =
                      snapshot.status === "active" ||
                      lastActivatedSnapshotId === snapshot.configSnapshotId;
                    return (
                      <>
                        <UiDataTable.Td>
                          {snapshot.configSnapshotId}
                        </UiDataTable.Td>
                        <UiDataTable.Td>
                          {projects.find(
                            (project) => project.id === snapshot.projectId,
                          )?.name ?? snapshot.projectId}
                        </UiDataTable.Td>
                        <UiDataTable.Td>{snapshot.revision}</UiDataTable.Td>
                        <UiDataTable.Td>
                          <UiChip
                            color={isActive ? "teal" : "gray"}
                            variant="light"
                          >
                            {isActive ? "active" : snapshot.status}
                          </UiChip>
                        </UiDataTable.Td>
                        <UiDataTable.Td>
                          {snapshot.routePolicyId}
                        </UiDataTable.Td>
                        <UiDataTable.Td>
                          {snapshot.providerResourceIds.join(", ") ||
                            "No provider assignments"}
                        </UiDataTable.Td>
                        <UiDataTable.Td>
                          {isActive
                            ? (snapshot.activatedAt ?? "activating now")
                            : (snapshot.activatedAt ?? "not yet activated")}
                        </UiDataTable.Td>
                        <UiDataTable.Td>
                          <UiButton
                            disabled={
                              isActive ||
                              activatingSnapshotId === snapshot.configSnapshotId
                            }
                            loading={
                              activatingSnapshotId === snapshot.configSnapshotId
                            }
                            onClick={() =>
                              void onActivate(snapshot.configSnapshotId)
                            }
                            size="sm"
                            variant="light"
                          >
                            Activate
                          </UiButton>
                        </UiDataTable.Td>
                      </>
                    );
                  })()}
                </UiDataTable.Tr>
              ))}
            </UiDataTable.Tbody>
          </UiDataTable>
        )}
      </UiSurface>
    </UiStack>
  );
}
