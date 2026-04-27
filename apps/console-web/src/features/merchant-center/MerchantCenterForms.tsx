import {
  UiAlert,
  UiButton,
  UiSurface,
  UiInline,
  UiStack,
  UiText,
  UiTextField,
  UiTextarea,
} from "@huge-router/ui-kit";
import {
  ActionStatusNotice,
  FieldErrorText,
} from "../control-plane/workflow-ui";
import type {
  FormErrors,
  ShopFormState,
  TrialConnectionFormState,
} from "./forms";

type StatusNoticeProps = {
  error: string | null;
  success: string | null;
  onDismiss: () => void;
};

type ShopAndTrialCardsProps = {
  shopErrors: FormErrors;
  shopForm: ShopFormState;
  trialErrors: FormErrors;
  trialForm: TrialConnectionFormState;
  isSubmittingShop: boolean;
  isSubmittingTrial: boolean;
  onCreateShop: () => Promise<void>;
  onCreateTrialConnection: () => Promise<void>;
  setShopForm: React.Dispatch<React.SetStateAction<ShopFormState>>;
  setTrialForm: React.Dispatch<React.SetStateAction<TrialConnectionFormState>>;
  statusNotice: StatusNoticeProps;
};

export function MerchantCenterIntro({
  statusNotice,
}: {
  statusNotice: StatusNoticeProps;
}) {
  return (
    <>
      <ActionStatusNotice
        error={statusNotice.error}
        onDismiss={statusNotice.onDismiss}
        success={statusNotice.success}
      />
      <UiAlert
        color="yellow"
        radius="md"
        title="Dedicated test key guardrail"
        variant="light"
      >
        Use a dedicated supplier test key only. Current relay evaluations are
        simulated evidence runs, not live upstream probes. Replay capsules let
        support and comparison reuse redacted evidence before live Probe ships.
      </UiAlert>
    </>
  );
}

export function ShopAndTrialCards({
  shopErrors,
  shopForm,
  trialErrors,
  trialForm,
  isSubmittingShop,
  isSubmittingTrial,
  onCreateShop,
  onCreateTrialConnection,
  setShopForm,
  setTrialForm,
}: ShopAndTrialCardsProps) {
  return (
    <UiInline align="stretch" grow>
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiStack>
          <UiText fw={700}>Create an evidence workspace</UiText>
          <UiTextField
            label="Evidence workspace id"
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
          <UiTextField
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
          <UiTextField
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
          <UiTextarea
            label="Announcement"
            minRows={2}
            onChange={(event) =>
              setShopForm((current) => ({
                ...current,
                announcement: event.currentTarget.value,
              }))
            }
            placeholder="Supplier evidence workspace for replay-backed evaluation"
            value={shopForm.announcement}
          />
          <UiInline justify="flex-end">
            <UiButton
              loading={isSubmittingShop}
              onClick={() => void onCreateShop()}
            >
              Create workspace
            </UiButton>
          </UiInline>
        </UiStack>
      </UiSurface>

      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiStack>
          <UiText fw={700}>Attach a supplier test endpoint</UiText>
          <UiTextField
            label="Test endpoint id"
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
          <UiTextField
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
          <UiTextField
            label="Endpoint base URL"
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
          <UiTextField
            label="Dedicated test API key"
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
          <UiTextField
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
          <UiTextarea
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
          <UiInline justify="flex-end">
            <UiButton
              loading={isSubmittingTrial}
              onClick={() => void onCreateTrialConnection()}
            >
              Save test endpoint
            </UiButton>
          </UiInline>
        </UiStack>
      </UiSurface>
    </UiInline>
  );
}
