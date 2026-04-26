import {
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
import { useState } from "react";
import { createFileRoute, useRouter } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import type { RoutePolicyView } from "../features/control-plane/types";
import {
  getConsoleDataService,
  getControlPlaneActionErrorMessage,
  type RoutePolicyMutationInput,
} from "../features/control-plane/service";
import {
  ActionStatusNotice,
  FieldErrorText,
  PROTOCOL_FAMILY_OPTIONS,
  splitCommaSeparatedValues,
  SUPPORTED_ROUTE_CAPABILITIES,
} from "../features/control-plane/workflow-ui";
import {
  getProtocolColor,
  getProtocolLabel,
  getProtocolOptionLabel,
  isPreviewProtocolFamily,
} from "../features/control-plane/protocol-display";

const protocolOrder = [
  "openai_chat",
  "anthropic_messages",
  "gemini_generate_content",
  "openai_responses",
  "mcp_streamable_http",
  "realtime_webrtc",
];

type RoutePoliciesPageData = {
  routePolicies: RoutePolicyView[];
};

type RoutePolicyFormState = {
  displayName: string;
  modelAlias: string;
  preferredRegions: string;
  protocolFamily: RoutePolicyMutationInput["protocolFamily"];
  requiredCapabilities: string;
  routePolicyId: string;
};

type RoutePolicyFormErrors = Partial<
  Record<keyof RoutePolicyFormState, string>
>;

function protocolDisplay(protocolFamily: string) {
  return getProtocolLabel(protocolFamily);
}

function protocolColor(protocolFamily: string) {
  return getProtocolColor(protocolFamily);
}

function routePoliciesByProtocol(routePolicies: RoutePolicyView[]) {
  return routePolicies.reduce<Record<string, RoutePolicyView[]>>(
    (acc, policy) => {
      const group = policy.protocolFamily;
      acc[group] ??= [];
      acc[group].push(policy);
      return acc;
    },
    {},
  );
}

function listSortedProtocols(groups: Record<string, RoutePolicyView[]>) {
  return Object.entries(groups).sort(([left], [right]) => {
    const leftIndex = protocolOrder.indexOf(left);
    const rightIndex = protocolOrder.indexOf(right);

    if (leftIndex >= 0 && rightIndex >= 0) {
      return leftIndex - rightIndex;
    }

    if (leftIndex >= 0) {
      return -1;
    }

    if (rightIndex >= 0) {
      return 1;
    }

    return left.localeCompare(right);
  });
}

function createEmptyRoutePolicyForm(): RoutePolicyFormState {
  return {
    displayName: "",
    modelAlias: "",
    preferredRegions: "",
    protocolFamily: "openai_chat",
    requiredCapabilities: "json_mode",
    routePolicyId: "",
  };
}

function routePolicyToFormState(policy: RoutePolicyView): RoutePolicyFormState {
  return {
    displayName: policy.name,
    modelAlias: policy.modelAlias,
    preferredRegions: policy.preferredRegions.join(", "),
    protocolFamily:
      policy.protocolFamily as RoutePolicyFormState["protocolFamily"],
    requiredCapabilities: policy.requiredCapabilities.join(", "),
    routePolicyId: policy.id,
  };
}

function validateRoutePolicyForm(form: RoutePolicyFormState) {
  const errors: RoutePolicyFormErrors = {};
  const capabilities = splitCommaSeparatedValues(form.requiredCapabilities);
  const unsupportedCapabilities = capabilities.filter(
    (capability) =>
      !SUPPORTED_ROUTE_CAPABILITIES.includes(
        capability as (typeof SUPPORTED_ROUTE_CAPABILITIES)[number],
      ),
  );

  if (!/^routepol_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.routePolicyId)) {
    errors.routePolicyId =
      "Use an id that starts with routepol_ and contains letters, digits, _ or -.";
  }

  if (!form.displayName.trim()) {
    errors.displayName = "Enter a route policy name.";
  }

  if (!form.modelAlias.trim()) {
    errors.modelAlias = "Enter a model alias.";
  }

  if (capabilities.length === 0) {
    errors.requiredCapabilities = "Enter at least one capability.";
  } else if (unsupportedCapabilities.length > 0) {
    errors.requiredCapabilities = `Unsupported capability values: ${unsupportedCapabilities.join(
      ", ",
    )}. Supported values: ${SUPPORTED_ROUTE_CAPABILITIES.join(", ")}.`;
  }

  return errors;
}

export const Route = createFileRoute("/app/routes")({
  loader: () =>
    loadRouteData(async () => {
      return {
        routePolicies: await getConsoleDataService().listRoutePolicies(),
      } as RoutePoliciesPageData;
    }),
  pendingComponent: () => <RouteLoadingState label="Loading routes" />,
  pendingMs: 0,
  component: RoutePoliciesPage,
});

function RoutePoliciesPage() {
  const result = Route.useLoaderData();
  const router = useRouter();
  const [editingPolicy, setEditingPolicy] = useState<RoutePolicyView | null>(
    null,
  );
  const [formErrors, setFormErrors] = useState<RoutePolicyFormErrors>({});
  const [formMode, setFormMode] = useState<"create" | "edit" | null>(null);
  const [formState, setFormState] = useState<RoutePolicyFormState>(
    createEmptyRoutePolicyForm(),
  );
  const [policyOverrides, setPolicyOverrides] = useState<
    Record<string, RoutePolicyView>
  >({});
  const [disabledPolicyIds, setDisabledPolicyIds] = useState<string[]>([]);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusSuccess, setStatusSuccess] = useState<string | null>(null);
  const [disablingPolicyId, setDisablingPolicyId] = useState<string | null>(
    null,
  );

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Review protocol-aware route policies and recent route diagnostics."
          title="Routes"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="Route policy and diagnostics data could not be loaded from the control-plane service."
          title="Routes unavailable"
        />
      </UiStack>
    );
  }

  const { routePolicies } = result.data;
  const effectiveRoutePolicies = routePolicies
    .filter((policy) => !disabledPolicyIds.includes(policy.id))
    .map((policy) => policyOverrides[policy.id] ?? policy);
  const groupedPolicies = routePoliciesByProtocol(effectiveRoutePolicies);
  const protocolGroups = listSortedProtocols(groupedPolicies);

  function updateField<K extends keyof RoutePolicyFormState>(
    key: K,
    value: RoutePolicyFormState[K],
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
    setEditingPolicy(null);
    setFormErrors({});
    setFormMode(null);
    setFormState(createEmptyRoutePolicyForm());
  }

  function openCreateForm() {
    setStatusError(null);
    setStatusSuccess(null);
    setEditingPolicy(null);
    setFormErrors({});
    setFormMode("create");
    setFormState(createEmptyRoutePolicyForm());
  }

  function openEditForm(policy: RoutePolicyView) {
    setStatusError(null);
    setStatusSuccess(null);
    setEditingPolicy(policy);
    setFormErrors({});
    setFormMode("edit");
    setFormState(routePolicyToFormState(policy));
  }

  async function onSubmit() {
    const errors = validateRoutePolicyForm(formState);
    setFormErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);

    const input: RoutePolicyMutationInput = {
      createdAt: editingPolicy?.createdAt,
      displayName: formState.displayName.trim(),
      modelAlias: formState.modelAlias.trim(),
      preferredRegions: splitCommaSeparatedValues(formState.preferredRegions),
      protocolFamily: formState.protocolFamily,
      requiredCapabilities: splitCommaSeparatedValues(
        formState.requiredCapabilities,
      ),
      routePolicyId: formState.routePolicyId.trim(),
      updatedAt: editingPolicy?.updatedAt,
      version: editingPolicy?.version,
    };

    try {
      if (formMode === "edit" && editingPolicy) {
        await getConsoleDataService().updateRoutePolicy(
          editingPolicy.id,
          input,
          editingPolicy.version,
        );
        setPolicyOverrides((current) => ({
          ...current,
          [editingPolicy.id]: {
            ...editingPolicy,
            modelAlias: input.modelAlias,
            name: input.displayName,
            preferredRegions: input.preferredRegions,
            protocolFamily: input.protocolFamily,
            requiredCapabilities: input.requiredCapabilities,
            updatedAt: new Date().toISOString(),
            version: editingPolicy.version + 1,
          },
        }));
        setStatusSuccess(`Updated route policy ${formState.displayName}.`);
      } else {
        await getConsoleDataService().createRoutePolicy(input);
        setStatusSuccess(`Created route policy ${formState.displayName}.`);
      }

      resetForm();
      await router.invalidate();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(
          error,
          formMode === "edit" ? "route-policy-update" : "route-policy-create",
        ),
      );
    } finally {
      setIsSubmitting(false);
    }
  }

  async function onDisable(policy: RoutePolicyView) {
    setDisablingPolicyId(policy.id);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      await getConsoleDataService().disableRoutePolicy(
        policy.id,
        policy.version,
      );
      setDisabledPolicyIds((current) => [...current, policy.id]);
      setStatusSuccess(`Disabled route policy ${policy.name}.`);
      await router.invalidate();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "route-policy-disable"),
      );
    } finally {
      setDisablingPolicyId(null);
    }
  }

  return (
    <UiStack>
      <PageHeader
        description="Review protocol-aware route policies and recent route diagnostics."
        title="Routes"
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
            {formMode === "edit" ? "Edit route policy" : "Create route policy"}
          </UiText>
          <UiInline>
            {formMode ? (
              <UiButton onClick={resetForm} size="sm" variant="subtle">
                Cancel
              </UiButton>
            ) : null}
            {!formMode ? (
              <UiButton onClick={openCreateForm} size="sm">
                Create route policy
              </UiButton>
            ) : null}
          </UiInline>
        </UiInline>
        {formMode ? (
          <UiStack>
            <UiTextField
              label="Route policy id"
              onChange={(event) =>
                updateField("routePolicyId", event.currentTarget.value)
              }
              placeholder="routepol_openai_chat_default"
              value={formState.routePolicyId}
            />
            <FieldErrorText error={formErrors.routePolicyId} />
            <UiInline grow>
              <UiTextField
                label="Display name"
                onChange={(event) =>
                  updateField("displayName", event.currentTarget.value)
                }
                placeholder="Acme Reasoning Fast"
                value={formState.displayName}
              />
              <UiTextField
                label="Model alias"
                onChange={(event) =>
                  updateField("modelAlias", event.currentTarget.value)
                }
                placeholder="reasoning-fast"
                value={formState.modelAlias}
              />
            </UiInline>
            <UiInline grow>
              <FieldErrorText error={formErrors.displayName} />
              <FieldErrorText error={formErrors.modelAlias} />
            </UiInline>
            <UiSelect
              data={PROTOCOL_FAMILY_OPTIONS.map((value) => ({
                label: getProtocolOptionLabel(value),
                value,
              }))}
              label="Protocol family"
              onChange={(value) =>
                updateField("protocolFamily", value ?? "openai_chat")
              }
              value={formState.protocolFamily}
            />
            <UiTextField
              description={`Supported capability values: ${SUPPORTED_ROUTE_CAPABILITIES.join(
                ", ",
              )}.`}
              label="Required capabilities"
              onChange={(event) =>
                updateField("requiredCapabilities", event.currentTarget.value)
              }
              placeholder="json_mode, tool_calling"
              value={formState.requiredCapabilities}
            />
            <FieldErrorText error={formErrors.requiredCapabilities} />
            <UiTextField
              description="Comma-separated regions are optional."
              label="Preferred regions"
              onChange={(event) =>
                updateField("preferredRegions", event.currentTarget.value)
              }
              placeholder="us-east-1, us-west-2"
              value={formState.preferredRegions}
            />
            <UiInline justify="flex-end">
              <UiButton loading={isSubmitting} onClick={() => void onSubmit()}>
                {formMode === "edit"
                  ? "Save route policy"
                  : "Create route policy"}
              </UiButton>
            </UiInline>
          </UiStack>
        ) : (
          <UiText c="dimmed" size="sm">
            Create protocol-aware route policies, edit existing definitions, and
            disable outdated ones.
          </UiText>
        )}
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Route policy config</UiText>
          <UiChip color="blue" variant="light">
            {effectiveRoutePolicies.length} policies
          </UiChip>
        </UiInline>
        {effectiveRoutePolicies.length === 0 ? (
          <EmptyCollectionState
            description="Create a route policy to define protocol-aware provider resolution."
            title="No route policies"
          />
        ) : (
          protocolGroups.map(([protocolLabel, policies]) => (
            <UiStack key={protocolLabel} gap="sm">
              <UiInline>
                <UiChip color={protocolColor(protocolLabel)} size="md">
                  {protocolDisplay(protocolLabel)}
                </UiChip>
                {isPreviewProtocolFamily(protocolLabel) ? (
                  <UiChip color="yellow" size="md" variant="light">
                    Preview
                  </UiChip>
                ) : null}
                <UiText c="dimmed" size="sm">
                  {policies.length} polic{policies.length === 1 ? "y" : "ies"}
                </UiText>
              </UiInline>
              <UiDataTable mb="md" striped withTableBorder>
                <UiDataTable.Thead>
                  <UiDataTable.Tr>
                    <UiDataTable.Th>Policy</UiDataTable.Th>
                    <UiDataTable.Th>Protocol</UiDataTable.Th>
                    <UiDataTable.Th>Model alias</UiDataTable.Th>
                    <UiDataTable.Th>Selected providers</UiDataTable.Th>
                    <UiDataTable.Th>Latest receipt</UiDataTable.Th>
                    <UiDataTable.Th>Preferred regions</UiDataTable.Th>
                    <UiDataTable.Th>Required capabilities</UiDataTable.Th>
                    <UiDataTable.Th>Version</UiDataTable.Th>
                    <UiDataTable.Th>Actions</UiDataTable.Th>
                  </UiDataTable.Tr>
                </UiDataTable.Thead>
                <UiDataTable.Tbody>
                  {policies.map((policy) => (
                    <UiDataTable.Tr key={policy.id}>
                      <UiDataTable.Td>{policy.name}</UiDataTable.Td>
                      <UiDataTable.Td>
                        <UiInline gap="xs">
                          <UiText>
                            {protocolDisplay(policy.protocolFamily)}
                          </UiText>
                          {isPreviewProtocolFamily(policy.protocolFamily) ? (
                            <UiChip color="yellow" size="sm" variant="light">
                              Preview
                            </UiChip>
                          ) : null}
                        </UiInline>
                      </UiDataTable.Td>
                      <UiDataTable.Td>{policy.modelAlias}</UiDataTable.Td>
                      <UiDataTable.Td>
                        {policy.selectedProviders.length > 0
                          ? policy.selectedProviders.join(", ")
                          : "Inactive snapshot"}
                      </UiDataTable.Td>
                      <UiDataTable.Td>
                        <UiStack gap={2}>
                          <UiChip
                            color={
                              policy.lastReceiptOutcome === "admitted"
                                ? "teal"
                                : policy.lastReceiptOutcome
                                  ? "orange"
                                  : "gray"
                            }
                            variant="light"
                          >
                            {policy.lastReceiptOutcome ?? "No receipt"}
                          </UiChip>
                          <UiText c="dimmed" size="sm">
                            {policy.lastFailureReason ??
                              "No recent routing failure recorded."}
                          </UiText>
                        </UiStack>
                      </UiDataTable.Td>
                      <UiDataTable.Td>
                        {policy.preferredRegions.join(", ") || "Any region"}
                      </UiDataTable.Td>
                      <UiDataTable.Td>
                        {policy.requiredCapabilities.join(", ")}
                      </UiDataTable.Td>
                      <UiDataTable.Td>{policy.version}</UiDataTable.Td>
                      <UiDataTable.Td>
                        <UiInline gap="xs">
                          <UiButton
                            component="a"
                            href={`/app/route-diagnostics/${policy.id}`}
                            size="xs"
                            variant="subtle"
                          >
                            Inspect
                          </UiButton>
                          <UiButton
                            onClick={() => openEditForm(policy)}
                            size="xs"
                            variant="light"
                          >
                            Edit
                          </UiButton>
                          <UiButton
                            color="red"
                            loading={disablingPolicyId === policy.id}
                            onClick={() => void onDisable(policy)}
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
            </UiStack>
          ))
        )}
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiText fw={700} mb="xs">
          Operator diagnostics
        </UiText>
        <UiText c="dimmed" size="sm">
          Use the inspect action to review protocol support, capability gaps,
          health blockers, and the recent receipt history for a specific route
          policy.
        </UiText>
        <UiInline mt="md">
          <UiButton
            component="a"
            href="/app/receipts"
            size="sm"
            variant="light"
          >
            Open route receipts
          </UiButton>
        </UiInline>
      </UiSurface>
    </UiStack>
  );
}
