import {
  Badge,
  Button,
  Card,
  Group,
  List,
  Select,
  Stack,
  Table,
  Text,
  TextInput,
} from "@mantine/core";
import { useState } from "react";
import { createFileRoute, useRouter } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import type {
  RouteReceiptDiagnosticView,
  RoutePolicyView,
} from "../features/control-plane/types";
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

const protocolLabelByFamily: Record<string, string> = {
  anthropic_messages: "Anthropic Messages",
  gemini_generate_content: "Gemini Generate Content",
  mcp_streamable_http: "MCP Streamable HTTP",
  openai_chat: "OpenAI Chat",
  openai_responses: "OpenAI Responses",
  realtime_webrtc: "Realtime WebRTC",
};

const protocolColorByFamily: Record<string, string> = {
  anthropic_messages: "orange",
  gemini_generate_content: "lime",
  mcp_streamable_http: "indigo",
  openai_chat: "blue",
  openai_responses: "violet",
  realtime_webrtc: "teal",
};

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
  routeReceipts: RouteReceiptDiagnosticView[];
};

type RoutePolicyFormState = {
  displayName: string;
  modelAlias: string;
  preferredRegions: string;
  protocolFamily: RoutePolicyMutationInput["protocolFamily"];
  requiredCapabilities: string;
  routePolicyId: string;
};

type RoutePolicyFormErrors = Partial<Record<keyof RoutePolicyFormState, string>>;

function protocolDisplay(protocolFamily: string) {
  return protocolLabelByFamily[protocolFamily] ?? protocolFamily;
}

function protocolColor(protocolFamily: string) {
  return protocolColorByFamily[protocolFamily] ?? "gray";
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
    protocolFamily: policy.protocolFamily as RoutePolicyFormState["protocolFamily"],
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
      const [routePolicies, routeReceipts] = await Promise.all([
        getConsoleDataService().listRoutePolicies(),
        getConsoleDataService().listRouteReceipts(),
      ]);

      return {
        routePolicies,
        routeReceipts,
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
      <Stack>
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
      </Stack>
    );
  }

  const { routePolicies, routeReceipts } = result.data;
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
      await getConsoleDataService().disableRoutePolicy(policy.id, policy.version);
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
    <Stack>
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
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>
            {formMode === "edit" ? "Edit route policy" : "Create route policy"}
          </Text>
          <Group>
            {formMode ? (
              <Button onClick={resetForm} size="sm" variant="subtle">
                Cancel
              </Button>
            ) : null}
            {!formMode ? (
              <Button onClick={openCreateForm} size="sm">
                Create route policy
              </Button>
            ) : null}
          </Group>
        </Group>
        {formMode ? (
          <Stack>
            <TextInput
              label="Route policy id"
              onChange={(event) =>
                updateField("routePolicyId", event.currentTarget.value)
              }
              placeholder="routepol_openai_chat_default"
              value={formState.routePolicyId}
            />
            <FieldErrorText error={formErrors.routePolicyId} />
            <Group grow>
              <TextInput
                label="Display name"
                onChange={(event) =>
                  updateField("displayName", event.currentTarget.value)
                }
                placeholder="Acme Reasoning Fast"
                value={formState.displayName}
              />
              <TextInput
                label="Model alias"
                onChange={(event) =>
                  updateField("modelAlias", event.currentTarget.value)
                }
                placeholder="reasoning-fast"
                value={formState.modelAlias}
              />
            </Group>
            <Group grow>
              <FieldErrorText error={formErrors.displayName} />
              <FieldErrorText error={formErrors.modelAlias} />
            </Group>
            <Select
              data={PROTOCOL_FAMILY_OPTIONS.map((value) => ({
                label: protocolDisplay(value),
                value,
              }))}
              label="Protocol family"
              onChange={(value) =>
                updateField(
                  "protocolFamily",
                  (value ??
                    "openai_chat") as RoutePolicyFormState["protocolFamily"],
                )
              }
              value={formState.protocolFamily}
            />
            <TextInput
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
            <TextInput
              description="Comma-separated regions are optional."
              label="Preferred regions"
              onChange={(event) =>
                updateField("preferredRegions", event.currentTarget.value)
              }
              placeholder="us-east-1, us-west-2"
              value={formState.preferredRegions}
            />
            <Group justify="flex-end">
              <Button loading={isSubmitting} onClick={() => void onSubmit()}>
                {formMode === "edit" ? "Save route policy" : "Create route policy"}
              </Button>
            </Group>
          </Stack>
        ) : (
          <Text c="dimmed" size="sm">
            Create protocol-aware route policies, edit existing definitions, and disable outdated ones.
          </Text>
        )}
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Route policy config</Text>
          <Badge color="blue" variant="light">
            {effectiveRoutePolicies.length} policies
          </Badge>
        </Group>
        {effectiveRoutePolicies.length === 0 ? (
          <EmptyCollectionState
            description="Create a route policy to define protocol-aware provider resolution."
            title="No route policies"
          />
        ) : (
          protocolGroups.map(([protocolLabel, policies]) => (
            <Stack key={protocolLabel} gap="sm">
              <Group>
                <Badge color={protocolColor(protocolLabel)} size="md">
                  {protocolLabelByFamily[protocolLabel] ?? protocolLabel}
                </Badge>
                <Text c="dimmed" size="sm">
                  {policies.length} polic{policies.length === 1 ? "y" : "ies"}
                </Text>
              </Group>
              <Table mb="md" striped withTableBorder>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>Policy</Table.Th>
                    <Table.Th>Protocol</Table.Th>
                    <Table.Th>Model alias</Table.Th>
                    <Table.Th>Selected providers</Table.Th>
                    <Table.Th>Preferred regions</Table.Th>
                    <Table.Th>Required capabilities</Table.Th>
                    <Table.Th>Version</Table.Th>
                    <Table.Th>Actions</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {policies.map((policy) => (
                    <Table.Tr key={policy.id}>
                      <Table.Td>{policy.name}</Table.Td>
                      <Table.Td>{protocolDisplay(policy.protocolFamily)}</Table.Td>
                      <Table.Td>{policy.modelAlias}</Table.Td>
                      <Table.Td>
                        {policy.selectedProviders.length > 0
                          ? policy.selectedProviders.join(", ")
                          : "Inactive snapshot"}
                      </Table.Td>
                      <Table.Td>
                        {policy.preferredRegions.join(", ") || "Any region"}
                      </Table.Td>
                      <Table.Td>
                        {policy.requiredCapabilities.join(", ")}
                      </Table.Td>
                      <Table.Td>{policy.version}</Table.Td>
                      <Table.Td>
                        <Group gap="xs">
                          <Button
                            onClick={() => openEditForm(policy)}
                            size="xs"
                            variant="light"
                          >
                            Edit
                          </Button>
                          <Button
                            color="red"
                            loading={disablingPolicyId === policy.id}
                            onClick={() => void onDisable(policy)}
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
            </Stack>
          ))
        )}
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Recent route receipts</Text>
          <Badge color="blue" variant="light">
            {routeReceipts.length}
          </Badge>
        </Group>
        {routeReceipts.length === 0 ? (
          <EmptyCollectionState
            description="No recent route receipts are available yet."
            title="No route receipts"
          />
        ) : (
          <Table striped withTableBorder>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Receipt ID</Table.Th>
                <Table.Th>Protocol</Table.Th>
                <Table.Th>Model alias</Table.Th>
                <Table.Th>Selected target</Table.Th>
                <Table.Th>Excluded targets</Table.Th>
                <Table.Th>Fallback transitions</Table.Th>
                <Table.Th>Diagnostics</Table.Th>
                <Table.Th>Normalized error</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {routeReceipts.map((receipt) => (
                <Table.Tr key={receipt.routeReceiptId}>
                  <Table.Td>{receipt.routeReceiptId}</Table.Td>
                  <Table.Td>{protocolDisplay(receipt.protocolFamily)}</Table.Td>
                  <Table.Td>{receipt.modelAlias}</Table.Td>
                  <Table.Td>{receipt.selectedTargetLabel}</Table.Td>
                  <Table.Td>
                    {receipt.excludedTargets.length === 0 ? (
                      <Text c="dimmed" size="sm">
                        None
                      </Text>
                    ) : (
                      <List size="sm" withPadding>
                        {receipt.excludedTargets.map((target) => (
                          <List.Item key={target.providerResourceId}>
                            {target.providerLabel} ({target.reason})
                          </List.Item>
                        ))}
                      </List>
                    )}
                  </Table.Td>
                  <Table.Td>
                    {receipt.fallbackTransitions.length === 0 ? (
                      <Text c="dimmed" size="sm">
                        None
                      </Text>
                    ) : (
                      <List size="sm" withPadding>
                        {receipt.fallbackTransitions.map((transition) => (
                          <List.Item
                            key={`${transition.fromProviderResourceId}-${transition.toProviderResourceId}-${transition.reason}`}
                          >
                            {transition.fromProviderLabel} &rarr;{" "}
                            {transition.toProviderLabel} ({transition.reason})
                          </List.Item>
                        ))}
                      </List>
                    )}
                  </Table.Td>
                  <Table.Td>
                    <Stack gap={2}>
                      {receipt.decisionTimeline.map((step) => (
                        <Text key={`${step.stage}-${step.message}`} size="sm">
                          {step.stage}: {step.message}
                        </Text>
                      ))}
                      {receipt.providerAttempts.map((attempt) => (
                        <Text
                          key={`${attempt.providerResourceId}-${attempt.attempt}`}
                          size="sm"
                        >
                          {attempt.providerLabel} attempt {attempt.attempt} (
                          {attempt.latencyMs}ms, {attempt.status})
                        </Text>
                      ))}
                      {receipt.policyChecks.map((check) => (
                        <Text key={check.policyId} size="sm">
                          {check.policyId}: {check.status}
                        </Text>
                      ))}
                    </Stack>
                  </Table.Td>
                  <Table.Td>
                    {receipt.normalizedError ? (
                      <Stack gap={2}>
                        <Text size="sm">{receipt.normalizedError.code}</Text>
                        <Text c="dimmed" size="sm">
                          {receipt.normalizedError.message}
                        </Text>
                      </Stack>
                    ) : (
                      <Text c="dimmed" size="sm">
                        None
                      </Text>
                    )}
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
