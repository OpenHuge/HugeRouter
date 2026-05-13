import {
  UiChip,
  UiButton,
  UiSurface,
  UiInline,
  UiSelect,
  UiStack,
  UiSwitch,
  UiText,
  UiTextField,
  UiTextarea,
} from "@huge-router/ui-kit";
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
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiInline justify="space-between">
          <UiText fw={700}>Legacy private inventory</UiText>
          <UiChip color="gray" variant="light">
            Legacy/private
          </UiChip>
        </UiInline>
        {shops.length === 0 ? (
          <EmptyCollectionState
            description="Create an evidence workspace first. Card-secret inventory is kept as a private legacy workflow, not the public ku0 product path."
            title="No evidence workspace yet"
          />
        ) : (
          <>
            <UiInline align="flex-start" grow>
              <UiStack gap="xs">
                <UiTextField
                  label="Card product id"
                  onChange={(event) =>
                    setCardForm((current) => ({
                      ...current,
                      cardProductId: event.currentTarget.value,
                    }))
                  }
                  placeholder="cardprod_legacy_trial_pack"
                  value={cardForm.cardProductId}
                />
                <FieldErrorText error={cardErrors.cardProductId} />
              </UiStack>
              <UiStack gap="xs">
                <UiSelect
                  data={shops.map((shop) => ({
                    label: `${shop.displayName} (${shop.merchantShopId})`,
                    value: shop.merchantShopId,
                  }))}
                  label="Evidence workspace"
                  onChange={(value) =>
                    setCardForm((current) => ({
                      ...current,
                      merchantShopId: value ?? "",
                    }))
                  }
                  value={cardForm.merchantShopId}
                />
                <FieldErrorText error={cardErrors.merchantShopId} />
              </UiStack>
              <UiStack gap="xs">
                <UiTextField
                  label="Project id"
                  onChange={(event) =>
                    setCardForm((current) => ({
                      ...current,
                      projectId: event.currentTarget.value,
                    }))
                  }
                  placeholder="proj_core"
                  value={cardForm.projectId}
                />
                <FieldErrorText error={cardErrors.projectId} />
              </UiStack>
            </UiInline>
            <UiInline align="flex-start" grow>
              <UiStack gap="xs">
                <UiTextField
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
              </UiStack>
              <UiStack gap="xs">
                <UiSelect
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
              </UiStack>
              <UiStack gap="xs">
                <UiSwitch
                  checked={cardForm.saleEnabled}
                  label="Enable WeChat sale"
                  onChange={(event) =>
                    setCardForm((current) => ({
                      ...current,
                      saleEnabled: event.currentTarget.checked,
                    }))
                  }
                />
              </UiStack>
            </UiInline>
            <UiTextarea
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
            <UiInline align="flex-start" grow>
              <UiStack gap="xs">
                <UiTextField
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
              </UiStack>
              <UiStack gap="xs">
                <UiTextField
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
              </UiStack>
              <UiStack gap="xs">
                <UiTextField
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
              </UiStack>
              <UiStack gap="xs">
                <UiTextField
                  label="CNY price cents"
                  onChange={(event) =>
                    setCardForm((current) => ({
                      ...current,
                      retailPriceCnyTotal: event.currentTarget.value,
                    }))
                  }
                  placeholder="199"
                  value={cardForm.retailPriceCnyTotal}
                />
                <FieldErrorText error={cardErrors.retailPriceCnyTotal} />
              </UiStack>
            </UiInline>
            <UiTextarea
              label="Delivery inventory ids"
              minRows={3}
              onChange={(event) =>
                setCardForm((current) => ({
                  ...current,
                  deliveryIds: event.currentTarget.value,
                }))
              }
              placeholder="delivery_abc123, delivery_def456"
              value={cardForm.deliveryIds}
            />
            <FieldErrorText error={cardErrors.deliveryIds} />
            <UiInline justify="flex-end">
              <UiButton
                loading={isSubmittingCard}
                onClick={() => void onCreateCardProduct()}
              >
                Create card product
              </UiButton>
            </UiInline>
          </>
        )}
      </UiStack>
    </UiSurface>
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
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiStack>
        <UiInline justify="space-between">
          <UiText fw={700}>Run supplier evidence check</UiText>
          <UiChip color="yellow" variant="light">
            Evidence source: simulated
          </UiChip>
        </UiInline>
        <UiText c="dimmed" size="sm">
          This Stage 1 runner records deterministic replay evidence. It is not a
          live upstream Probe result and must stay labeled as simulated.
        </UiText>
        {trialConnections.length === 0 ? (
          <EmptyCollectionState
            description="Attach at least one supplier test endpoint before recording replay-backed evidence."
            title="No supplier test endpoint yet"
          />
        ) : (
          <UiInline align="flex-end">
            <UiSelect
              data={trialConnections.map((connection) => ({
                label: `${connection.providerLabel} (${connection.trialConnectionId})`,
                value: connection.trialConnectionId,
              }))}
              label="Supplier test endpoint"
              onChange={(value) =>
                setEvaluationForm({
                  trialConnectionId: value ?? "",
                })
              }
              value={evaluationForm.trialConnectionId}
            />
            <UiButton
              loading={isSubmittingEvaluation}
              onClick={() => void onRunEvaluation()}
            >
              Run evidence check
            </UiButton>
          </UiInline>
        )}
      </UiStack>
    </UiSurface>
  );
}
