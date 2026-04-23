import {
  Alert,
  Badge,
  Button,
  Card,
  Group,
  Select,
  Stack,
  Table,
  Text,
  TextInput,
  Textarea,
} from "@mantine/core";
import { useEffect, useState } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { PageHeader } from "@huge-router/ui-kit";
import { loadRouteData } from "../features/control-plane/loaders";
import {
  EmptyCollectionState,
  RouteErrorState,
  RouteLoadingState,
} from "../features/control-plane/route-state";
import {
  getConsoleDataService,
  getControlPlaneActionErrorMessage,
} from "../features/control-plane/service";
import type {
  CardProductView,
  MerchantShopView,
  MerchantWorkspaceData,
  ReplayCapsuleView,
  TrialConnectionView,
} from "../features/control-plane/types";
import {
  ActionStatusNotice,
  FieldErrorText,
} from "../features/control-plane/workflow-ui";

type ShopFormState = {
  merchantShopId: string;
  slug: string;
  displayName: string;
  announcement: string;
};

type CardProductFormState = {
  cardProductId: string;
  merchantShopId: string;
  title: string;
  description: string;
  inventoryCount: string;
  faceValueUsd: string;
  retailPriceUsd: string;
  supportsTrial: string;
};

type TrialConnectionFormState = {
  trialConnectionId: string;
  providerLabel: string;
  endpointBaseUrl: string;
  apiKey: string;
  targetModel: string;
  notes: string;
};

type EvaluationFormState = {
  trialConnectionId: string;
};

type FormErrors = Record<string, string | undefined>;

function createShopForm(): ShopFormState {
  return {
    announcement: "",
    displayName: "",
    merchantShopId: "",
    slug: "",
  };
}

function createCardProductForm(): CardProductFormState {
  return {
    cardProductId: "",
    description: "",
    faceValueUsd: "",
    inventoryCount: "10",
    merchantShopId: "",
    retailPriceUsd: "",
    supportsTrial: "true",
    title: "",
  };
}

function createTrialConnectionForm(): TrialConnectionFormState {
  return {
    apiKey: "",
    endpointBaseUrl: "",
    notes: "",
    providerLabel: "",
    targetModel: "",
    trialConnectionId: "",
  };
}

function createEvaluationForm(): EvaluationFormState {
  return {
    trialConnectionId: "",
  };
}

function validateShopForm(form: ShopFormState): FormErrors {
  const errors: FormErrors = {};

  if (!/^mshop_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.merchantShopId)) {
    errors.merchantShopId = "Use an id with the mshop_ prefix.";
  }

  if (!/^[a-z0-9-]+$/.test(form.slug)) {
    errors.slug = "Use a lowercase slug with digits or hyphens.";
  }

  if (!form.displayName.trim()) {
    errors.displayName = "Enter a shop display name.";
  }

  return errors;
}

function validateCardProductForm(form: CardProductFormState): FormErrors {
  const errors: FormErrors = {};

  if (!/^cardprod_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.cardProductId)) {
    errors.cardProductId = "Use an id with the cardprod_ prefix.";
  }

  if (!form.merchantShopId) {
    errors.merchantShopId = "Select a merchant shop.";
  }

  if (!form.title.trim()) {
    errors.title = "Enter a product title.";
  }

  if (!form.description.trim()) {
    errors.description = "Enter a product description.";
  }

  if (!/^\d+(\.\d+)?$/.test(form.faceValueUsd)) {
    errors.faceValueUsd = "Enter a numeric face value.";
  }

  if (!/^\d+(\.\d+)?$/.test(form.retailPriceUsd)) {
    errors.retailPriceUsd = "Enter a numeric retail price.";
  }

  if (!/^\d+$/.test(form.inventoryCount)) {
    errors.inventoryCount = "Enter a non-negative inventory count.";
  }

  return errors;
}

function validateTrialConnectionForm(form: TrialConnectionFormState): FormErrors {
  const errors: FormErrors = {};

  if (!/^trialconn_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.trialConnectionId)) {
    errors.trialConnectionId = "Use an id with the trialconn_ prefix.";
  }

  if (!form.providerLabel.trim()) {
    errors.providerLabel = "Enter a provider label.";
  }

  if (!form.endpointBaseUrl.startsWith("https://")) {
    errors.endpointBaseUrl = "Use an https endpoint.";
  }

  if (form.apiKey.trim().length < 8) {
    errors.apiKey = "Enter a dedicated trial API key.";
  }

  if (!form.targetModel.trim()) {
    errors.targetModel = "Enter a target model.";
  }

  return errors;
}

export const Route = createFileRoute("/app/merchant")({
  loader: () =>
    loadRouteData(async () => getConsoleDataService().getMerchantWorkspace()),
  pendingComponent: () => <RouteLoadingState label="Loading merchant center" />,
  pendingMs: 0,
  component: MerchantCenterPage,
});

function MerchantCenterPage() {
  const result = Route.useLoaderData();

  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Manage small-shop setup, card-secret listings, trial relay connections, and replay-backed evaluations."
          title="Merchant Center"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result?.message : undefined}
          description="Merchant data could not be loaded from the control-plane service."
          title="Merchant center unavailable"
        />
      </Stack>
    );
  }

  const [workspace, setWorkspace] = useState<MerchantWorkspaceData>(result.data);
  const [statusError, setStatusError] = useState<string | null>(null);
  const [statusSuccess, setStatusSuccess] = useState<string | null>(null);
  const [shopErrors, setShopErrors] = useState<FormErrors>({});
  const [cardErrors, setCardErrors] = useState<FormErrors>({});
  const [trialErrors, setTrialErrors] = useState<FormErrors>({});
  const [shopForm, setShopForm] = useState<ShopFormState>(createShopForm());
  const [cardForm, setCardForm] = useState<CardProductFormState>(
    createCardProductForm(),
  );
  const [trialForm, setTrialForm] = useState<TrialConnectionFormState>(
    createTrialConnectionForm(),
  );
  const [evaluationForm, setEvaluationForm] = useState<EvaluationFormState>(
    createEvaluationForm(),
  );
  const [isSubmittingShop, setIsSubmittingShop] = useState(false);
  const [isSubmittingCard, setIsSubmittingCard] = useState(false);
  const [isSubmittingTrial, setIsSubmittingTrial] = useState(false);
  const [isSubmittingEvaluation, setIsSubmittingEvaluation] = useState(false);
  const [isLoadingReplayCapsule, setIsLoadingReplayCapsule] = useState(false);
  const [selectedReplayCapsule, setSelectedReplayCapsule] =
    useState<ReplayCapsuleView | null>(null);
  const [selectedReplayCapsuleId, setSelectedReplayCapsuleId] = useState<
    string | null
  >(null);

  useEffect(() => {
    setWorkspace(result.data);
  }, [result.data]);

  useEffect(() => {
    setCardForm((current) => ({
      ...current,
      merchantShopId:
        current.merchantShopId || result.data.shops[0]?.merchantShopId || "",
    }));
    setEvaluationForm((current) => ({
      trialConnectionId:
        current.trialConnectionId ||
        result.data.trialConnections[0]?.trialConnectionId ||
        "",
    }));
  }, [result.data.shops, result.data.trialConnections]);

  async function refreshWorkspace() {
    setWorkspace(await getConsoleDataService().getMerchantWorkspace());
  }

  async function onViewReplayCapsule(replayCapsuleId: string) {
    setIsLoadingReplayCapsule(true);
    setSelectedReplayCapsuleId(replayCapsuleId);
    setStatusError(null);

    try {
      const capsule =
        await getConsoleDataService().getReplayCapsule(replayCapsuleId);
      setSelectedReplayCapsule(capsule);
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "relay-evaluation-create"),
      );
    } finally {
      setIsLoadingReplayCapsule(false);
    }
  }

  async function onCreateShop() {
    const errors = validateShopForm(shopForm);
    setShopErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmittingShop(true);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      const created = await getConsoleDataService().createMerchantShop({
        announcement: shopForm.announcement.trim() || undefined,
        displayName: shopForm.displayName.trim(),
        merchantShopId: shopForm.merchantShopId.trim(),
        slug: shopForm.slug.trim(),
      });
      setStatusSuccess(`Created merchant shop ${created.displayName}.`);
      setShopForm(createShopForm());
      await refreshWorkspace();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "merchant-shop-create"),
      );
    } finally {
      setIsSubmittingShop(false);
    }
  }

  async function onCreateCardProduct() {
    const errors = validateCardProductForm(cardForm);
    setCardErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmittingCard(true);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      const created = await getConsoleDataService().createCardProduct({
        cardProductId: cardForm.cardProductId.trim(),
        description: cardForm.description.trim(),
        faceValueUsd: cardForm.faceValueUsd.trim(),
        inventoryCount: Number(cardForm.inventoryCount),
        merchantShopId: cardForm.merchantShopId,
        retailPriceUsd: cardForm.retailPriceUsd.trim(),
        supportsTrial: cardForm.supportsTrial === "true",
        title: cardForm.title.trim(),
      });
      setStatusSuccess(`Created card product ${created.title}.`);
      setCardForm(createCardProductForm());
      await refreshWorkspace();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "card-product-create"),
      );
    } finally {
      setIsSubmittingCard(false);
    }
  }

  async function onCreateTrialConnection() {
    const errors = validateTrialConnectionForm(trialForm);
    setTrialErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmittingTrial(true);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      const created = await getConsoleDataService().createTrialConnection({
        apiKey: trialForm.apiKey.trim(),
        endpointBaseUrl: trialForm.endpointBaseUrl.trim(),
        notes: trialForm.notes.trim() || undefined,
        providerLabel: trialForm.providerLabel.trim(),
        targetModel: trialForm.targetModel.trim(),
        trialConnectionId: trialForm.trialConnectionId.trim(),
      });
      setStatusSuccess(`Connected trial provider ${created.providerLabel}.`);
      setTrialForm(createTrialConnectionForm());
      setEvaluationForm({ trialConnectionId: created.trialConnectionId });
      await refreshWorkspace();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "trial-connection-create"),
      );
    } finally {
      setIsSubmittingTrial(false);
    }
  }

  async function onRunEvaluation() {
    if (!evaluationForm.trialConnectionId) {
      setStatusError("Select a trial connection before starting an evaluation.");
      return;
    }

    setIsSubmittingEvaluation(true);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      const evaluation = await getConsoleDataService().runRelayEvaluation({
        trialConnectionId: evaluationForm.trialConnectionId,
      });
      setStatusSuccess(
        `Recorded evaluation ${evaluation.relayEvaluationId} with replay capsule ${evaluation.replayCapsuleId}.`,
      );
      await refreshWorkspace();
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "relay-evaluation-create"),
      );
    } finally {
      setIsSubmittingEvaluation(false);
    }
  }

  return (
    <Stack>
      <PageHeader
        description="Operate a small shop, attach dedicated trial relays, and keep replay-backed evidence so repeated review does not burn live tokens."
        title="Merchant Center"
      />
      <ActionStatusNotice
        error={statusError}
        onDismiss={() => {
          setStatusError(null);
          setStatusSuccess(null);
        }}
        success={statusSuccess}
      />
      <Alert color="yellow" radius="md" title="Trial key guardrail" variant="light">
        Use a dedicated trial key only. Merchant evaluations now record replayable data so support and comparison can reuse prior runs instead of repeatedly spending live tokens.
      </Alert>

      <Group align="stretch" grow>
        <Card padding="lg" radius="md" shadow="sm">
          <Stack>
            <Text fw={700}>Open a shop</Text>
            <TextInput
              label="Merchant shop id"
              onChange={(event) =>
                setShopForm((current) => ({
                  ...current,
                  merchantShopId: event.currentTarget.value,
                }))
              }
              placeholder="mshop_acme"
              value={shopForm.merchantShopId}
            />
            <FieldErrorText error={shopErrors.merchantShopId} />
            <TextInput
              label="Slug"
              onChange={(event) =>
                setShopForm((current) => ({
                  ...current,
                  slug: event.currentTarget.value,
                }))
              }
              placeholder="acme-small-shop"
              value={shopForm.slug}
            />
            <FieldErrorText error={shopErrors.slug} />
            <TextInput
              label="Display name"
              onChange={(event) =>
                setShopForm((current) => ({
                  ...current,
                  displayName: event.currentTarget.value,
                }))
              }
              placeholder="Acme Small Shop"
              value={shopForm.displayName}
            />
            <FieldErrorText error={shopErrors.displayName} />
            <Textarea
              label="Announcement"
              minRows={2}
              onChange={(event) =>
                setShopForm((current) => ({
                  ...current,
                  announcement: event.currentTarget.value,
                }))
              }
              placeholder="Fresh trial cards with replay-backed evaluation"
              value={shopForm.announcement}
            />
            <Group justify="flex-end">
              <Button loading={isSubmittingShop} onClick={onCreateShop}>
                Create shop
              </Button>
            </Group>
          </Stack>
        </Card>

        <Card padding="lg" radius="md" shadow="sm">
          <Stack>
            <Text fw={700}>Attach a trial relay</Text>
            <TextInput
              label="Trial connection id"
              onChange={(event) =>
                setTrialForm((current) => ({
                  ...current,
                  trialConnectionId: event.currentTarget.value,
                }))
              }
              placeholder="trialconn_acme"
              value={trialForm.trialConnectionId}
            />
            <FieldErrorText error={trialErrors.trialConnectionId} />
            <TextInput
              label="Provider label"
              onChange={(event) =>
                setTrialForm((current) => ({
                  ...current,
                  providerLabel: event.currentTarget.value,
                }))
              }
              placeholder="Acme Relay"
              value={trialForm.providerLabel}
            />
            <FieldErrorText error={trialErrors.providerLabel} />
            <TextInput
              label="Endpoint"
              onChange={(event) =>
                setTrialForm((current) => ({
                  ...current,
                  endpointBaseUrl: event.currentTarget.value,
                }))
              }
              placeholder="https://relay.example.com/v1"
              value={trialForm.endpointBaseUrl}
            />
            <FieldErrorText error={trialErrors.endpointBaseUrl} />
            <TextInput
              label="Trial API key"
              onChange={(event) =>
                setTrialForm((current) => ({
                  ...current,
                  apiKey: event.currentTarget.value,
                }))
              }
              placeholder="sk-trial-..."
              value={trialForm.apiKey}
            />
            <FieldErrorText error={trialErrors.apiKey} />
            <TextInput
              label="Target model"
              onChange={(event) =>
                setTrialForm((current) => ({
                  ...current,
                  targetModel: event.currentTarget.value,
                }))
              }
              placeholder="claude-sonnet"
              value={trialForm.targetModel}
            />
            <FieldErrorText error={trialErrors.targetModel} />
            <Textarea
              label="Notes"
              minRows={2}
              onChange={(event) =>
                setTrialForm((current) => ({
                  ...current,
                  notes: event.currentTarget.value,
                }))
              }
              placeholder="Dedicated trial key only"
              value={trialForm.notes}
            />
            <Group justify="flex-end">
              <Button loading={isSubmittingTrial} onClick={onCreateTrialConnection}>
                Save trial relay
              </Button>
            </Group>
          </Stack>
        </Card>
      </Group>

      <Card padding="lg" radius="md" shadow="sm">
        <Stack>
          <Text fw={700}>Add a card product</Text>
          {workspace.shops.length === 0 ? (
            <EmptyCollectionState
              description="Create a shop first, then attach card-secret products for that storefront."
              title="No merchant shop yet"
            />
          ) : (
            <>
              <Group align="flex-start" grow>
                <Stack gap="xs">
                  <TextInput
                    label="Card product id"
                    onChange={(event) =>
                      setCardForm((current) => ({
                        ...current,
                        cardProductId: event.currentTarget.value,
                      }))
                    }
                    placeholder="cardprod_trial_pack"
                    value={cardForm.cardProductId}
                  />
                  <FieldErrorText error={cardErrors.cardProductId} />
                </Stack>
                <Stack gap="xs">
                  <Select
                    data={workspace.shops.map((shop) => ({
                      label: `${shop.displayName} (${shop.merchantShopId})`,
                      value: shop.merchantShopId,
                    }))}
                    label="Merchant shop"
                    onChange={(value) =>
                      setCardForm((current) => ({
                        ...current,
                        merchantShopId: value ?? "",
                      }))
                    }
                    value={cardForm.merchantShopId}
                  />
                  <FieldErrorText error={cardErrors.merchantShopId} />
                </Stack>
              </Group>
              <Group align="flex-start" grow>
                <Stack gap="xs">
                  <TextInput
                    label="Title"
                    onChange={(event) =>
                      setCardForm((current) => ({
                        ...current,
                        title: event.currentTarget.value,
                      }))
                    }
                    placeholder="Claude Trial Pack"
                    value={cardForm.title}
                  />
                  <FieldErrorText error={cardErrors.title} />
                </Stack>
                <Stack gap="xs">
                  <Select
                    data={[
                      { label: "Trial enabled", value: "true" },
                      { label: "Regular product", value: "false" },
                    ]}
                    label="Product mode"
                    onChange={(value) =>
                      setCardForm((current) => ({
                        ...current,
                        supportsTrial: value ?? "true",
                      }))
                    }
                    value={cardForm.supportsTrial}
                  />
                </Stack>
              </Group>
              <Textarea
                label="Description"
                minRows={2}
                onChange={(event) =>
                  setCardForm((current) => ({
                    ...current,
                    description: event.currentTarget.value,
                  }))
                }
                placeholder="Starter batch for relay verification"
                value={cardForm.description}
              />
              <FieldErrorText error={cardErrors.description} />
              <Group align="flex-start" grow>
                <Stack gap="xs">
                  <TextInput
                    label="Inventory count"
                    onChange={(event) =>
                      setCardForm((current) => ({
                        ...current,
                        inventoryCount: event.currentTarget.value,
                      }))
                    }
                    value={cardForm.inventoryCount}
                  />
                  <FieldErrorText error={cardErrors.inventoryCount} />
                </Stack>
                <Stack gap="xs">
                  <TextInput
                    label="Face value USD"
                    onChange={(event) =>
                      setCardForm((current) => ({
                        ...current,
                        faceValueUsd: event.currentTarget.value,
                      }))
                    }
                    placeholder="1.00"
                    value={cardForm.faceValueUsd}
                  />
                  <FieldErrorText error={cardErrors.faceValueUsd} />
                </Stack>
                <Stack gap="xs">
                  <TextInput
                    label="Retail price USD"
                    onChange={(event) =>
                      setCardForm((current) => ({
                        ...current,
                        retailPriceUsd: event.currentTarget.value,
                      }))
                    }
                    placeholder="1.99"
                    value={cardForm.retailPriceUsd}
                  />
                  <FieldErrorText error={cardErrors.retailPriceUsd} />
                </Stack>
              </Group>
              <Group justify="flex-end">
                <Button loading={isSubmittingCard} onClick={onCreateCardProduct}>
                  Create card product
                </Button>
              </Group>
            </>
          )}
        </Stack>
      </Card>

      <Card padding="lg" radius="md" shadow="sm">
        <Stack>
          <Group justify="space-between">
            <Text fw={700}>Run relay evaluation</Text>
            <Badge color="blue" variant="light">
              Runner: simulated
            </Badge>
          </Group>
          <Text c="dimmed" size="sm">
            Each run records a replay capsule so follow-up review can reuse captured evidence instead of repeatedly spending live tokens.
          </Text>
          {workspace.trialConnections.length === 0 ? (
            <EmptyCollectionState
              description="Attach at least one trial relay before running replay-backed evaluation."
              title="No trial relay yet"
            />
          ) : (
            <Group align="flex-end">
              <Select
                data={workspace.trialConnections.map((connection) => ({
                  label: `${connection.providerLabel} (${connection.trialConnectionId})`,
                  value: connection.trialConnectionId,
                }))}
                label="Trial connection"
                onChange={(value) =>
                  setEvaluationForm({
                    trialConnectionId: value ?? "",
                  })
                }
                value={evaluationForm.trialConnectionId}
              />
              <Button
                loading={isSubmittingEvaluation}
                onClick={onRunEvaluation}
              >
                Run evaluation
              </Button>
            </Group>
          )}
        </Stack>
      </Card>

      <MerchantShopTable shops={workspace.shops} />
      <CardProductTable products={workspace.cardProducts} />
      <TrialConnectionTable connections={workspace.trialConnections} />
      <RelayEvaluationTable
        isLoadingReplayCapsule={isLoadingReplayCapsule}
        onViewReplayCapsule={onViewReplayCapsule}
        selectedReplayCapsuleId={selectedReplayCapsuleId}
        workspace={workspace}
      />
      <ReplayCapsuleDetailCard replayCapsule={selectedReplayCapsule} />
    </Stack>
  );
}

function MerchantShopTable({ shops }: { shops: MerchantShopView[] }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Shops</Text>
        {shops.length === 0 ? (
          <EmptyCollectionState
            description="Open your first small shop to start listing card-secret products."
            title="No shops"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Shop</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Slug</Table.Th>
                <Table.Th>Fulfillment</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {shops.map((shop) => (
                <Table.Tr key={shop.merchantShopId}>
                  <Table.Td>
                    <Text fw={600}>{shop.displayName}</Text>
                    <Text c="dimmed" size="sm">
                      {shop.merchantShopId}
                    </Text>
                  </Table.Td>
                  <Table.Td>{shop.status}</Table.Td>
                  <Table.Td>{shop.slug}</Table.Td>
                  <Table.Td>{shop.fulfillmentMode}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}

function CardProductTable({ products }: { products: CardProductView[] }) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Card Products</Text>
        {products.length === 0 ? (
          <EmptyCollectionState
            description="Create a card-secret product after your merchant shop is ready."
            title="No card products"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Product</Table.Th>
                <Table.Th>Status</Table.Th>
                <Table.Th>Inventory</Table.Th>
                <Table.Th>Face Value</Table.Th>
                <Table.Th>Retail Price</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {products.map((product) => (
                <Table.Tr key={product.cardProductId}>
                  <Table.Td>
                    <Text fw={600}>{product.title}</Text>
                    <Text c="dimmed" size="sm">
                      {product.cardProductId}
                    </Text>
                  </Table.Td>
                  <Table.Td>{product.status}</Table.Td>
                  <Table.Td>{product.inventoryCount}</Table.Td>
                  <Table.Td>${product.faceValueUsd}</Table.Td>
                  <Table.Td>${product.retailPriceUsd}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}

function TrialConnectionTable({
  connections,
}: {
  connections: TrialConnectionView[];
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Trial Connections</Text>
        {connections.length === 0 ? (
          <EmptyCollectionState
            description="Attach a dedicated test relay before running evaluation."
            title="No trial connections"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Endpoint</Table.Th>
                <Table.Th>Masked Key</Table.Th>
                <Table.Th>Model</Table.Th>
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {connections.map((connection) => (
                <Table.Tr key={connection.trialConnectionId}>
                  <Table.Td>
                    <Text fw={600}>{connection.providerLabel}</Text>
                    <Text c="dimmed" size="sm">
                      {connection.trialConnectionId}
                    </Text>
                  </Table.Td>
                  <Table.Td>{connection.endpointBaseUrl}</Table.Td>
                  <Table.Td>{connection.apiKeyMasked}</Table.Td>
                  <Table.Td>{connection.targetModel}</Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}

function RelayEvaluationTable({
  isLoadingReplayCapsule,
  onViewReplayCapsule,
  selectedReplayCapsuleId,
  workspace,
}: {
  isLoadingReplayCapsule: boolean;
  onViewReplayCapsule: (replayCapsuleId: string) => void;
  selectedReplayCapsuleId: string | null;
  workspace: MerchantWorkspaceData;
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Recent Evaluations</Text>
        {workspace.recentEvaluations.length === 0 ? (
          <EmptyCollectionState
            description="Run your first evaluation to produce replay-backed merchant evidence."
            title="No evaluations"
          />
        ) : (
          <Table striped withRowBorders>
            <Table.Thead>
              <Table.Tr>
                <Table.Th>Provider</Table.Th>
                <Table.Th>Verdict</Table.Th>
                <Table.Th>Score</Table.Th>
                <Table.Th>Replay Capsule</Table.Th>
                <Table.Th>Saved Tokens</Table.Th>
                <Table.Th />
              </Table.Tr>
            </Table.Thead>
            <Table.Tbody>
              {workspace.recentEvaluations.map((evaluation) => (
                <Table.Tr key={evaluation.relayEvaluationId}>
                  <Table.Td>
                    <Text fw={600}>{evaluation.providerLabel}</Text>
                    <Text c="dimmed" size="sm">
                      {evaluation.summary}
                    </Text>
                  </Table.Td>
                  <Table.Td>{evaluation.verdict}</Table.Td>
                  <Table.Td>{evaluation.overallScore}</Table.Td>
                  <Table.Td>
                    <Text>{evaluation.replayCapsuleId}</Text>
                    <Text c="dimmed" size="sm">
                      {evaluation.runnerMode}
                    </Text>
                  </Table.Td>
                  <Table.Td>{evaluation.estimatedTokensSaved}</Table.Td>
                  <Table.Td>
                    <Button
                      loading={
                        isLoadingReplayCapsule &&
                        selectedReplayCapsuleId === evaluation.replayCapsuleId
                      }
                      onClick={() =>
                        onViewReplayCapsule(evaluation.replayCapsuleId)
                      }
                      size="xs"
                      variant="light"
                    >
                      View replay
                    </Button>
                  </Table.Td>
                </Table.Tr>
              ))}
            </Table.Tbody>
          </Table>
        )}
      </Stack>
    </Card>
  );
}

function ReplayCapsuleDetailCard({
  replayCapsule,
}: {
  replayCapsule: ReplayCapsuleView | null;
}) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Group justify="space-between">
          <Text fw={700}>Replay Capsule</Text>
          {replayCapsule ? (
            <Badge color="teal" variant="light">
              {replayCapsule.redactionTier}
            </Badge>
          ) : null}
        </Group>
        {!replayCapsule ? (
          <EmptyCollectionState
            description="Open a replay capsule from the evaluation table to inspect the redacted request shape and upstream error hint."
            title="No replay capsule selected"
          />
        ) : (
          <>
            <Group grow>
              <Card padding="md" radius="md" withBorder>
                <Text c="dimmed" size="sm">
                  Replay Capsule ID
                </Text>
                <Text fw={600}>{replayCapsule.replayCapsuleId}</Text>
              </Card>
              <Card padding="md" radius="md" withBorder>
                <Text c="dimmed" size="sm">
                  Request / Trace
                </Text>
                <Text fw={600}>{replayCapsule.requestId}</Text>
                <Text c="dimmed" size="sm">
                  {replayCapsule.traceId}
                </Text>
              </Card>
              <Card padding="md" radius="md" withBorder>
                <Text c="dimmed" size="sm">
                  Receipt / Snapshot
                </Text>
                <Text fw={600}>{replayCapsule.routeReceiptId}</Text>
                <Text c="dimmed" size="sm">
                  {replayCapsule.configSnapshotId}
                </Text>
              </Card>
            </Group>
            <Table striped withRowBorders>
              <Table.Tbody>
                <Table.Tr>
                  <Table.Th>Protocol Family</Table.Th>
                  <Table.Td>
                    {
                      replayCapsule.normalizedRequestSummary.protocolFamily
                    }
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th>Model Alias</Table.Th>
                  <Table.Td>
                    {replayCapsule.normalizedRequestSummary.modelAlias}
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th>Estimated Prompt Tokens</Table.Th>
                  <Table.Td>
                    {
                      replayCapsule.normalizedRequestSummary
                        .estimatedPromptTokens
                    }
                  </Table.Td>
                </Table.Tr>
                <Table.Tr>
                  <Table.Th>Upstream Error Hint</Table.Th>
                  <Table.Td>
                    {replayCapsule.upstreamErrorCode ?? "none"}
                  </Table.Td>
                </Table.Tr>
              </Table.Tbody>
            </Table>
          </>
        )}
      </Stack>
    </Card>
  );
}
