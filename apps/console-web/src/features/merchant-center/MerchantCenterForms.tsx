import {
  Alert,
  Button,
  Card,
  Group,
  Stack,
  Text,
  TextInput,
  Textarea,
} from "@mantine/core";
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
      <Alert
        color="yellow"
        radius="md"
        title="Trial key guardrail"
        variant="light"
      >
        Use a dedicated trial key only. Merchant evaluations now record
        replayable data so support and comparison can reuse prior runs instead
        of repeatedly spending live tokens.
      </Alert>
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
            <Button
              loading={isSubmittingShop}
              onClick={() => void onCreateShop()}
            >
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
            <Button
              loading={isSubmittingTrial}
              onClick={() => void onCreateTrialConnection()}
            >
              Save trial relay
            </Button>
          </Group>
        </Stack>
      </Card>
    </Group>
  );
}
