import {
  UiChip,
  UiSurface,
  UiInline,
  UiStack,
  UiDataTable,
  UiText,
} from "@huge-router/ui-kit";
import { Link, createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import {
  getProtocolLabel,
  isPreviewProtocolFamily,
} from "../features/control-plane/protocol-display";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/app/route-diagnostics/$routePolicyId")({
  loader: ({ params }) =>
    loadRouteData(() =>
      getConsoleDataService().getRouteDiagnostics(params.routePolicyId),
    ),
  pendingComponent: () => (
    <RouteLoadingState label="Loading route diagnostics" />
  ),
  pendingMs: 0,
  component: RouteDiagnosticsPage,
});

function RouteDiagnosticsPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Explain how the active snapshot and recorded receipts affected target selection for this route."
          title="Route diagnostics"
        />
        <RouteErrorState
          description="The route diagnostics drill-down could not be loaded from the control-plane service."
          title="Diagnostics unavailable"
        />
      </UiStack>
    );
  }

  const { activeSnapshotId, activeSnapshotMatchesRoutePolicy, diagnostics } =
    result.data;

  return (
    <UiStack>
      <PageHeader
        description="Explain how the active snapshot and recorded receipts affected target selection for this route."
        title={diagnostics.route_policy.display_name}
      />
      <UiInline justify="space-between">
        <UiStack gap={2}>
          <UiInline gap="xs">
            <UiText c="dimmed" size="sm">
              {getProtocolLabel(diagnostics.route_policy.protocol_family)} ·{" "}
              {diagnostics.route_policy.model_alias}
            </UiText>
            {isPreviewProtocolFamily(
              diagnostics.route_policy.protocol_family,
            ) ? (
              <UiChip color="yellow" size="sm" variant="light">
                Preview
              </UiChip>
            ) : null}
          </UiInline>
          <UiText c="dimmed" size="sm">
            Required capabilities:{" "}
            {diagnostics.route_policy.required_capabilities.join(", ")}
          </UiText>
        </UiStack>
        <UiChip
          color={activeSnapshotMatchesRoutePolicy ? "teal" : "gray"}
          variant="light"
        >
          {activeSnapshotMatchesRoutePolicy
            ? `active snapshot ${activeSnapshotId}`
            : activeSnapshotId
              ? `active snapshot ${activeSnapshotId} is routing a different policy`
              : "no active snapshot"}
        </UiChip>
      </UiInline>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Target decision matrix</UiText>
          <UiChip color="blue" variant="light">
            {diagnostics.targets.length} targets
          </UiChip>
        </UiInline>
        <UiDataTable striped withTableBorder>
          <UiDataTable.Thead>
            <UiDataTable.Tr>
              <UiDataTable.Th>Target</UiDataTable.Th>
              <UiDataTable.Th>Decision</UiDataTable.Th>
              <UiDataTable.Th>Protocol</UiDataTable.Th>
              <UiDataTable.Th>Capability gaps</UiDataTable.Th>
              <UiDataTable.Th>Health</UiDataTable.Th>
              <UiDataTable.Th>Reason</UiDataTable.Th>
            </UiDataTable.Tr>
          </UiDataTable.Thead>
          <UiDataTable.Tbody>
            {diagnostics.targets.map((target) => (
              <UiDataTable.Tr
                key={target.provider_resource.provider_resource_id}
              >
                <UiDataTable.Td>
                  <UiStack gap={2}>
                    <UiInline gap="xs">
                      <UiText fw={600}>{target.provider_resource.name}</UiText>
                      {target.provider_resource.is_transit_gateway ? (
                        <UiChip color="indigo" variant="light">
                          transit
                        </UiChip>
                      ) : null}
                    </UiInline>
                    <UiText c="dimmed" size="sm">
                      {target.provider_resource.provider_id} ·{" "}
                      {target.provider_resource.region}
                    </UiText>
                  </UiStack>
                </UiDataTable.Td>
                <UiDataTable.Td>
                  <UiChip
                    color={decisionColor(target.decision)}
                    variant="light"
                  >
                    {target.decision}
                  </UiChip>
                </UiDataTable.Td>
                <UiDataTable.Td>
                  <UiChip
                    color={target.supports_protocol_family ? "teal" : "red"}
                    variant="light"
                  >
                    {target.supports_protocol_family
                      ? "supported"
                      : "unsupported"}
                  </UiChip>
                </UiDataTable.Td>
                <UiDataTable.Td>
                  {target.capability_gaps.length > 0
                    ? target.capability_gaps.join(", ")
                    : "None"}
                </UiDataTable.Td>
                <UiDataTable.Td>
                  <UiStack gap={2}>
                    <UiChip
                      color={healthColor(target.provider_resource.health_state)}
                      variant="light"
                    >
                      {target.provider_resource.health_state}
                    </UiChip>
                    <UiText c="dimmed" size="sm">
                      {target.provider_resource.quarantine_reason ??
                        target.provider_resource.health_message ??
                        "No health message recorded."}
                    </UiText>
                  </UiStack>
                </UiDataTable.Td>
                <UiDataTable.Td>
                  <UiStack gap={2}>
                    <UiText size="sm">{target.reason}</UiText>
                    <UiText c="dimmed" size="sm">
                      {target.in_active_snapshot
                        ? "Included in active snapshot"
                        : "Not in active snapshot set"}
                    </UiText>
                    {target.recent_receipt_id ? (
                      <UiText c="dimmed" size="sm">
                        Receipt: {target.recent_receipt_id}
                      </UiText>
                    ) : null}
                  </UiStack>
                </UiDataTable.Td>
              </UiDataTable.Tr>
            ))}
          </UiDataTable.Tbody>
        </UiDataTable>
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Recent receipts</UiText>
          <UiChip color="gray" variant="light">
            {diagnostics.recent_receipts.length}
          </UiChip>
        </UiInline>
        {diagnostics.recent_receipts.length === 0 ? (
          <UiText c="dimmed" size="sm">
            No recent receipts for this route policy.
          </UiText>
        ) : (
          <UiStack gap="xs">
            {diagnostics.recent_receipts.map((receipt) => (
              <UiSurface
                key={receipt.route_receipt_id}
                padding="md"
                radius="md"
                withBorder
              >
                <UiInline justify="space-between">
                  <UiStack gap={2}>
                    <UiText fw={600}>{receipt.route_receipt_id}</UiText>
                    <UiText c="dimmed" size="sm">
                      {receipt.created_at}
                    </UiText>
                  </UiStack>
                  <UiChip
                    color={
                      receipt.admission_result === "admitted"
                        ? "teal"
                        : "orange"
                    }
                    variant="light"
                  >
                    {receipt.admission_result}
                  </UiChip>
                </UiInline>
                <UiText c="dimmed" mt="xs" size="sm">
                  {receipt.failure_reason ?? "No failure reason recorded."}
                </UiText>
              </UiSurface>
            ))}
          </UiStack>
        )}
      </UiSurface>
      <UiText c="dimmed" size="sm">
        Return to <Link to="/app/routes">Routes</Link> or{" "}
        <Link to="/app/receipts">Receipts</Link>.
      </UiText>
    </UiStack>
  );
}

function decisionColor(decision: string) {
  switch (decision) {
    case "selected":
      return "teal";
    case "eligible":
      return "blue";
    default:
      return "orange";
  }
}

function healthColor(healthState: string) {
  switch (healthState) {
    case "healthy":
      return "teal";
    case "degraded":
      return "yellow";
    case "quarantined":
      return "red";
    case "draining":
      return "orange";
    default:
      return "gray";
  }
}
