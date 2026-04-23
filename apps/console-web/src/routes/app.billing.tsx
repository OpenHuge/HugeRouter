import {
  Badge,
  Button,
  ButtonGroup,
  Card,
  Group,
  Stack,
  Table,
  Text,
} from "@mantine/core";
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import { getConsoleDataService } from "../features/control-plane/service";

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
  const [latestJob, setLatestJob] = useState(
    result && result.state !== "error"
      ? (result.data.exportJobs[0] ?? null)
      : null,
  );
  const [queueingExport, setQueueingExport] = useState(false);

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Track balance projections, threshold status, and the latest billing export jobs."
          title="Billing"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result.message : undefined}
          description="Billing projections could not be loaded from the control-plane service."
          title="Billing unavailable"
        />
      </Stack>
    );
  }

  const data = result.data;
  const exportJobs = latestJob
    ? [
        latestJob,
        ...data.exportJobs.filter(
          (job) => job.exportJobId !== latestJob.exportJobId,
        ),
      ]
    : data.exportJobs;

  return (
    <Stack>
      <PageHeader
        description="Track balance projections, threshold status, and the latest billing export jobs."
        title="Billing"
      />
      <ButtonGroup>
        {(["7d", "30d", "90d"] as const).map((range) => (
          <Button
            color={search.range === range ? "blue" : "gray"}
            component="a"
            href={`/app/billing?range=${range}${search.projectId ? `&projectId=${search.projectId}` : ""}`}
            key={range}
            variant={search.range === range ? "filled" : "light"}
          >
            {range}
          </Button>
        ))}
      </ButtonGroup>
      <ButtonGroup>
        <Button
          color={!data.activeProjectId ? "blue" : "gray"}
          component="a"
          href={`/app/billing?range=${search.range}`}
          variant={!data.activeProjectId ? "filled" : "light"}
        >
          all projects
        </Button>
        {data.availableProjects.map((project) => (
          <Button
            color={data.activeProjectId === project.id ? "blue" : "gray"}
            component="a"
            href={`/app/billing?range=${search.range}&projectId=${project.id}`}
            key={project.id}
            variant={data.activeProjectId === project.id ? "filled" : "light"}
          >
            {project.name}
          </Button>
        ))}
      </ButtonGroup>
      <Group grow>
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
      </Group>
      <Group grow>
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
      </Group>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Projection status</Text>
          <Badge
            color={
              data.thresholdStatus === "exceeded"
                ? "red"
                : data.thresholdStatus === "warning"
                  ? "yellow"
                  : "teal"
            }
            variant="light"
          >
            {data.thresholdStatus}
          </Badge>
        </Group>
        <Stack gap="xs">
          <Text>Last projected at: {data.lastProjectedAt}</Text>
          <Button
            loading={queueingExport}
            onClick={() => {
              void (async () => {
                setQueueingExport(true);
                try {
                  const job = await getConsoleDataService().queueBillingExport(
                    search.range,
                    search.projectId,
                  );
                  setLatestJob(job);
                } finally {
                  setQueueingExport(false);
                }
              })();
            }}
            variant="light"
          >
            Queue export
          </Button>
        </Stack>
      </Card>
      <Card padding="lg" radius="md" shadow="sm">
        <Group justify="space-between" mb="md">
          <Text fw={700}>Export jobs</Text>
          <Badge color="blue" variant="light">
            {exportJobs.length}
          </Badge>
        </Group>
        <Table striped withTableBorder>
          <Table.Thead>
            <Table.Tr>
              <Table.Th>Export job</Table.Th>
              <Table.Th>Status</Table.Th>
              <Table.Th>Format</Table.Th>
              <Table.Th>Requested</Table.Th>
              <Table.Th>Completed</Table.Th>
            </Table.Tr>
          </Table.Thead>
          <Table.Tbody>
            {exportJobs.map((job) => (
              <Table.Tr key={job.exportJobId}>
                <Table.Td>{job.exportJobId}</Table.Td>
                <Table.Td>{job.status}</Table.Td>
                <Table.Td>{job.format}</Table.Td>
                <Table.Td>{job.requestedAt}</Table.Td>
                <Table.Td>{job.completedAt ?? "—"}</Table.Td>
              </Table.Tr>
            ))}
          </Table.Tbody>
        </Table>
      </Card>
    </Stack>
  );
}

function MetricCard({ label, value }: { label: string; value: string }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Text c="dimmed" size="sm">
        {label}
      </Text>
      <Text fw={700} mt="xs" size="lg">
        {value}
      </Text>
    </Card>
  );
}
