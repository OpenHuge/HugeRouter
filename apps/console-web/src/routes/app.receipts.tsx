import {
  UiChip,
  UiSurface,
  UiInline,
  UiStack,
  UiDataTable,
  UiText,
} from "@huge-router/ui-kit";
import { createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

export const Route = createFileRoute("/app/receipts")({
  loader: () =>
    loadRouteData(() => getConsoleDataService().listRouteReceipts()),
  pendingComponent: () => <RouteLoadingState label="Loading route receipts" />,
  pendingMs: 0,
  component: RouteReceiptsPage,
});

function RouteReceiptsPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Inspect recent route receipts, failures, and target exclusion reasons."
          title="Route receipts"
        />
        <RouteErrorState
          description="Recent route receipts could not be loaded from the control-plane service."
          title="Receipts unavailable"
        />
      </UiStack>
    );
  }

  const receipts = result.data;

  return (
    <UiStack>
      <PageHeader
        description="Inspect recent route receipts, failures, and target exclusion reasons."
        title="Route receipts"
      />
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Recent receipts</UiText>
          <UiChip color="blue" variant="light">
            {receipts.length}
          </UiChip>
        </UiInline>
        {receipts.length === 0 ? (
          <EmptyCollectionState
            description="Route receipts will appear after the first routed request is recorded."
            title="No route receipts"
          />
        ) : (
          <UiDataTable striped withTableBorder>
            <UiDataTable.Thead>
              <UiDataTable.Tr>
                <UiDataTable.Th>Receipt</UiDataTable.Th>
                <UiDataTable.Th>Route</UiDataTable.Th>
                <UiDataTable.Th>Result</UiDataTable.Th>
                <UiDataTable.Th>Selected target</UiDataTable.Th>
                <UiDataTable.Th>Failure or exclusion reason</UiDataTable.Th>
              </UiDataTable.Tr>
            </UiDataTable.Thead>
            <UiDataTable.Tbody>
              {receipts.map((receipt) => (
                <UiDataTable.Tr key={receipt.receiptId}>
                  <UiDataTable.Td>
                    <UiStack gap={2}>
                      <UiText fw={600}>{receipt.receiptId}</UiText>
                      <UiText c="dimmed" size="sm">
                        {receipt.createdAt}
                      </UiText>
                    </UiStack>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiStack gap={2}>
                      <UiText fw={600}>{receipt.routeName}</UiText>
                      <UiText c="dimmed" size="sm">
                        {receipt.protocolFamily} · {receipt.modelAlias}
                      </UiText>
                    </UiStack>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip
                      color={
                        receipt.admissionResult === "admitted"
                          ? "teal"
                          : "orange"
                      }
                      variant="light"
                    >
                      {receipt.admissionResult}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    {receipt.selectedTargetName ?? "No selected target"}
                  </UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiStack gap={2}>
                      <UiText size="sm">
                        {receipt.failureReason ??
                          receipt.excludedTargets[0]?.reason ??
                          "Successful receipt with no exclusions."}
                      </UiText>
                      {receipt.excludedTargets.length > 0 ? (
                        <UiText c="dimmed" size="sm">
                          Excluded:{" "}
                          {receipt.excludedTargets
                            .map((target) => target.reason_code)
                            .join(", ")}
                        </UiText>
                      ) : null}
                    </UiStack>
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
