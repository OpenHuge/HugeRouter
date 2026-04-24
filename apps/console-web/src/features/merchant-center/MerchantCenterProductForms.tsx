import {
  Badge,
  Button,
  Card,
  Group,
  Select,
  Stack,
  Text,
  TextInput,
  Textarea,
} from "@mantine/core";
import type {
  MerchantShopView,
  TrialConnectionView,
} from "../control-plane/types";
import { EmptyCollectionState } from "../control-plane/route-state";
import { FieldErrorText } from "../control-plane/workflow-ui";
import type {
  CardProductFormState,
  EvaluationFormState,
  FormErrors,
} from "./forms";

type CardProductCardProps = {
  cardErrors: FormErrors;
  cardForm: CardProductFormState;
  isSubmittingCard: boolean;
  onCreateCardProduct: () => Promise<void>;
  setCardForm: React.Dispatch<React.SetStateAction<CardProductFormState>>;
  shops: MerchantShopView[];
};

type RelayEvaluationCardProps = {
  evaluationForm: EvaluationFormState;
  isSubmittingEvaluation: boolean;
  onRunEvaluation: () => Promise<void>;
  setEvaluationForm: React.Dispatch<React.SetStateAction<EvaluationFormState>>;
  trialConnections: TrialConnectionView[];
};

export function CardProductCard({
  cardErrors,
  cardForm,
  isSubmittingCard,
  onCreateCardProduct,
  setCardForm,
  shops,
}: CardProductCardProps) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Text fw={700}>Add a card product</Text>
        {shops.length === 0 ? (
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
                  data={shops.map((shop) => ({
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
              <Button
                loading={isSubmittingCard}
                onClick={() => void onCreateCardProduct()}
              >
                Create card product
              </Button>
            </Group>
          </>
        )}
      </Stack>
    </Card>
  );
}

export function RelayEvaluationCard({
  evaluationForm,
  isSubmittingEvaluation,
  onRunEvaluation,
  setEvaluationForm,
  trialConnections,
}: RelayEvaluationCardProps) {
  return (
    <Card padding="lg" radius="md" shadow="sm">
      <Stack>
        <Group justify="space-between">
          <Text fw={700}>Run relay evaluation</Text>
          <Badge color="blue" variant="light">
            Runner: simulated
          </Badge>
        </Group>
        <Text c="dimmed" size="sm">
          Each run records a replay capsule so follow-up review can reuse
          captured evidence instead of repeatedly spending live tokens.
        </Text>
        {trialConnections.length === 0 ? (
          <EmptyCollectionState
            description="Attach at least one trial relay before running replay-backed evaluation."
            title="No trial relay yet"
          />
        ) : (
          <Group align="flex-end">
            <Select
              data={trialConnections.map((connection) => ({
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
              onClick={() => void onRunEvaluation()}
            >
              Run evaluation
            </Button>
          </Group>
        )}
      </Stack>
    </Card>
  );
}
