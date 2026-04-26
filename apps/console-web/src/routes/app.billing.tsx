import {
  UiChip,
  UiButton,
  UiButtonGroup,
  UiSurface,
  UiInline,
  UiStack,
  UiDataTable,
  UiText,
} from "@huge-router/ui-kit";
import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import {
  getConsoleDataService,
  getControlPlaneActionErrorMessage,
} from "../features/control-plane/service";
import type { BillingExportJobView } from "../features/control-plane/types";
import { ActionStatusNotice } from "../features/control-plane/workflow-ui";

type BillingSearch = {
  projectId?: string;
  range: "7d" | "30d" | "90d";
};

function parseBillingSearch(rawSearch: Record<string, unknown>): BillingSearch {
  return {
    projectId:
      typeof rawSearch.projectId === "string" && rawSearch.projectId.length > 0
        ? rawSearch.projectId
        : undefined,
    range:
      rawSearch.range === "7d" || rawSearch.range === "90d"
        ? rawSearch.range
        : "30d",
  };
}

function triggerExportDownload(content: string, filename: string) {
  const blob = new Blob([content], { type: "text/csv;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");

  anchor.href = url;
  anchor.download = filename;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  URL.revokeObjectURL(url);
}

function thresholdStatusLabel(status: string) {
  switch (status) {
    case "ok":
      return "Within budget";
    case "warning":
      return "Approaching budget";
    case "exceeded":
      return "Budget exceeded";
    default:
      return status;
  }
}

export const Route = createFileRoute("/app/billing")({
  validateSearch: parseBillingSearch,
  loaderDeps: ({ search }) => search,
  loader: ({ deps }) =>
    loadRouteData(() =>
      getConsoleDataService().getBillingDashboard(deps.range, deps.projectId),
    ),
  pendingComponent: () => <RouteLoadingState label="Loading billing" />,
  pendingMs: 0,
  component: BillingPage,
});

function BillingPage() {
  const result = Route.useLoaderData();
  const search = Route.useSearch();
  const [downloadingJobId, setDownloadingJobId] = useState<string | null>(null);
  const [queueingExport, setQueueingExport] = useState(false);
  const [refreshingExports, setRefreshingExports] = useState(false);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusSuccess, setStatusSuccess] = useState<string | null>(null);

  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Review persisted spend totals, budget threshold state, and recorded billing export jobs."
          title="Billing"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result.message : undefined}
          description="Persisted billing totals could not be loaded from the control-plane service."
          title="Billing unavailable"
        />
      </UiStack>
    );
  }

  const data = result.data;
  const [exportJobs, setExportJobs] = useState<BillingExportJobView[]>(
    data.exportJobs,
  );

  useEffect(() => {
    setExportJobs(data.exportJobs);
  }, [data.exportJobs]);

  const effectiveExportJobs = exportJobs;

  async function refreshBillingExports() {
    setRefreshingExports(true);
    try {
      const refreshed = await getConsoleDataService().getBillingDashboard(
        search.range,
        search.projectId,
      );
      setExportJobs(refreshed.exportJobs);
    } finally {
      setRefreshingExports(false);
    }
  }

  async function onQueueExport() {
    setQueueingExport(true);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      await getConsoleDataService().queueBillingExport(
        search.range,
        search.projectId,
      );
      setStatusSuccess("Queued a billing export job.");
      await refreshBillingExports();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "billing-export-queue"),
      );
    } finally {
      setQueueingExport(false);
    }
  }

  async function onDownloadExport(exportJobId: string) {
    setDownloadingJobId(exportJobId);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      const content =
        await getConsoleDataService().downloadBillingExport(exportJobId);
      triggerExportDownload(content, `${exportJobId}.csv`);
      setStatusSuccess(`Downloaded export ${exportJobId}.csv.`);
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "billing-export-download"),
      );
    } finally {
      setDownloadingJobId(null);
    }
  }

  return (
    <UiStack>
      <PageHeader
        description="Review persisted spend totals, budget threshold state, and recorded billing export jobs."
        title="Billing"
      />
      <ActionStatusNotice
        error={statusError}
        onDismiss={() => {
          setStatusError(null);
          setStatusSuccess(null);
        }}
        success={statusSuccess}
      />
      <UiButtonGroup>
        {(["7d", "30d", "90d"] as const).map((range) => (
          <UiButton
            color={search.range === range ? "blue" : "gray"}
            component="a"
            href={`/app/billing?range=${range}${search.projectId ? `&projectId=${search.projectId}` : ""}`}
            key={range}
            variant={search.range === range ? "filled" : "light"}
          >
            {range}
          </UiButton>
        ))}
      </UiButtonGroup>
      <UiButtonGroup>
        <UiButton
          color={!data.activeProjectId ? "blue" : "gray"}
          component="a"
          href={`/app/billing?range=${search.range}`}
          variant={!data.activeProjectId ? "filled" : "light"}
        >
          all projects
        </UiButton>
        {data.availableProjects.map((project) => (
          <UiButton
            color={data.activeProjectId === project.id ? "blue" : "gray"}
            component="a"
            href={`/app/billing?range=${search.range}&projectId=${project.id}`}
            key={project.id}
            variant={data.activeProjectId === project.id ? "filled" : "light"}
          >
            {project.name}
          </UiButton>
        ))}
      </UiButtonGroup>
      <UiInline grow>
        <MetricCard label="Range" value={data.rangeLabel} />
        <MetricCard
          label="Project scope"
          value={
            data.availableProjects.find(
              (project) => project.id === data.activeProjectId,
            )?.name ?? "All projects"
          }
        />
        <MetricCard
          label="Provider cost total"
          value={`$${data.providerCostTotalUsd}`}
        />
        <MetricCard
          label="Billable total"
          value={`$${data.billableTotalUsd}`}
        />
      </UiInline>
      <UiInline grow>
        <MetricCard
          label="Configured budget"
          value={`$${data.configuredBudgetUsd}`}
        />
        <MetricCard
          label="Remaining budget"
          value={`$${data.remainingBudgetUsd}`}
        />
        <MetricCard
          label="Projection lag"
          value={`${data.projectionLagSeconds}s`}
        />
      </UiInline>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Projection status</UiText>
          <UiChip
            color={
              data.thresholdStatus === "exceeded"
                ? "red"
                : data.thresholdStatus === "warning"
                  ? "yellow"
                  : "teal"
            }
            variant="light"
          >
            {thresholdStatusLabel(data.thresholdStatus)}
          </UiChip>
        </UiInline>
        <UiStack gap="xs">
          <UiText>Projection updated at: {data.lastProjectedAt}</UiText>
          <UiInline>
            <UiButton
              loading={queueingExport}
              onClick={() => void onQueueExport()}
              variant="light"
            >
              Queue export
            </UiButton>
            <UiButton
              loading={refreshingExports}
              onClick={() => void refreshBillingExports()}
              variant="subtle"
            >
              Refresh jobs
            </UiButton>
          </UiInline>
        </UiStack>
      </UiSurface>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiText fw={700}>Billing export jobs</UiText>
          <UiChip color="blue" variant="light">
            {effectiveExportJobs.length}
          </UiChip>
        </UiInline>
        <UiDataTable striped withTableBorder>
          <UiDataTable.Thead>
            <UiDataTable.Tr>
              <UiDataTable.Th>Export job</UiDataTable.Th>
              <UiDataTable.Th>Status</UiDataTable.Th>
              <UiDataTable.Th>Format</UiDataTable.Th>
              <UiDataTable.Th>Requested</UiDataTable.Th>
              <UiDataTable.Th>Completed</UiDataTable.Th>
              <UiDataTable.Th>Error</UiDataTable.Th>
              <UiDataTable.Th>Actions</UiDataTable.Th>
            </UiDataTable.Tr>
          </UiDataTable.Thead>
          <UiDataTable.Tbody>
            {effectiveExportJobs.map((job) => {
              const isCompleted = job.status === "completed";
              const isFailed =
                job.status === "failed" || job.status === "error";
              const isPending = !isCompleted && !isFailed;

              return (
                <UiDataTable.Tr key={job.exportJobId}>
                  <UiDataTable.Td>{job.exportJobId}</UiDataTable.Td>
                  <UiDataTable.Td>
                    <UiChip
                      color={isCompleted ? "teal" : isFailed ? "red" : "yellow"}
                      variant="light"
                    >
                      {job.status}
                    </UiChip>
                  </UiDataTable.Td>
                  <UiDataTable.Td>{job.format}</UiDataTable.Td>
                  <UiDataTable.Td>{job.requestedAt}</UiDataTable.Td>
                  <UiDataTable.Td>{job.completedAt ?? "—"}</UiDataTable.Td>
                  <UiDataTable.Td>{job.errorMessage ?? "—"}</UiDataTable.Td>
                  <UiDataTable.Td>
                    {isCompleted ? (
                      <UiButton
                        loading={downloadingJobId === job.exportJobId}
                        onClick={() => void onDownloadExport(job.exportJobId)}
                        size="xs"
                        variant="light"
                      >
                        Download
                      </UiButton>
                    ) : isPending ? (
                      <UiText c="dimmed" size="sm">
                        Pending
                      </UiText>
                    ) : (
                      <UiText c="red" size="sm">
                        Retry unavailable
                      </UiText>
                    )}
                  </UiDataTable.Td>
                </UiDataTable.Tr>
              );
            })}
          </UiDataTable.Tbody>
        </UiDataTable>
      </UiSurface>
    </UiStack>
  );
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiText c="dimmed" size="sm">
        {label}
      </UiText>
      <UiText fw={700} mt="xs" size="lg">
        {value}
      </UiText>
    </UiSurface>
  );
}
