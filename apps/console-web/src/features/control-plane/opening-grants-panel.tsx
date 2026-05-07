import {
  UiAlert,
  UiButton,
  UiChip,
  UiDataTable,
  UiInline,
  UiSelect,
  UiStack,
  UiSurface,
  UiText,
  UiTextField,
} from "@huge-router/ui-kit";
import { useEffect, useState } from "react";
import type {
  ConfigSnapshotView,
  OpeningGrantView,
} from "./types";
import {
  buildOpeningGrantCreateInput,
  createOpeningGrantFormState,
  countActiveOpeningGrants,
  summarizeOpeningGrantOwners,
  type OpeningGrantFormErrors,
  type OpeningGrantFormState,
  validateOpeningGrantForm,
} from "./opening-grants-view-model";
import {
  getConsoleDataService,
  getControlPlaneActionErrorMessage,
} from "./service";
import { EmptyCollectionState } from "./route-state";
import { FieldErrorText } from "./workflow-ui";

type OpeningGrantSecretPreview = {
  grantId: string;
  granteeId: string;
  keyPrefix: string;
  secretValue?: string;
};

export function OpeningGrantsPanel({
  initialOpeningGrants,
  snapshots,
  setStatusError,
  setStatusSuccess,
}: {
  initialOpeningGrants: OpeningGrantView[];
  snapshots: ConfigSnapshotView[];
  setStatusError: (message: string | null) => void;
  setStatusSuccess: (message: string | null) => void;
}) {
  const [formErrors, setFormErrors] = useState<OpeningGrantFormErrors>({});
  const [formOpen, setFormOpen] = useState(false);
  const [formState, setFormState] = useState<OpeningGrantFormState>(() =>
    createOpeningGrantFormState([]),
  );
  const [grants, setGrants] = useState<OpeningGrantView[]>(
    () => initialOpeningGrants,
  );
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [secretPreview, setSecretPreview] =
    useState<OpeningGrantSecretPreview | null>(null);
  const [revokingGrantId, setRevokingGrantId] = useState<string | null>(null);

  useEffect(() => {
    setGrants(initialOpeningGrants);
  }, [initialOpeningGrants]);

  const ownerSummaries = summarizeOpeningGrantOwners(grants);
  const selectedProjectId = snapshots.find(
    (snapshot) => snapshot.configSnapshotId === formState.configSnapshotId,
  )?.projectId;
  const selectedOwnerActiveCount = formState.ownerAccountId.trim()
    ? countActiveOpeningGrants(grants, {
        ownerAccountId: formState.ownerAccountId.trim(),
        projectId: selectedProjectId,
      })
    : null;

  function openForm() {
    setFormErrors({});
    setFormOpen(true);
    setFormState(createOpeningGrantFormState(snapshots));
  }

  function resetForm() {
    setFormErrors({});
    setFormOpen(false);
    setFormState(createOpeningGrantFormState(snapshots));
  }

  function updateField<K extends keyof OpeningGrantFormState>(
    key: K,
    value: OpeningGrantFormState[K],
  ) {
    setFormState((current) => ({ ...current, [key]: value }));
    setFormErrors((current) => ({ ...current, [key]: undefined }));
  }

  async function onCreate() {
    const errors = validateOpeningGrantForm(formState, {
      existingGrants: grants,
      snapshots,
    });
    setFormErrors(errors);

    if (Object.keys(errors).length > 0) {
      return;
    }

    setIsSubmitting(true);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      const created = await getConsoleDataService().createOpeningGrant(
        buildOpeningGrantCreateInput(formState),
      );

      setSecretPreview({
        grantId: created.grant.grantId,
        granteeId: created.grant.granteeId,
        keyPrefix: created.keyPrefix,
        secretValue: created.plaintext,
      });
      setStatusSuccess(`Created child key for ${created.grant.granteeId}.`);
      resetForm();
      setGrants(await getConsoleDataService().listOpeningGrants());
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "opening-grant-create"),
      );
    } finally {
      setIsSubmitting(false);
    }
  }

  async function onRevoke(grantId: string, version: number) {
    setRevokingGrantId(grantId);
    setStatusError(null);
    setStatusSuccess(null);

    try {
      await getConsoleDataService().revokeOpeningGrant(grantId, version);
      setStatusSuccess(`Revoked child key ${grantId}.`);
      setGrants(await getConsoleDataService().listOpeningGrants());
    } catch (error) {
      setStatusError(
        getControlPlaneActionErrorMessage(error, "opening-grant-revoke"),
      );
    } finally {
      setRevokingGrantId(null);
    }
  }

  return (
    <>
      {secretPreview ? (
        <UiAlert
          color="yellow"
          radius="md"
          title="Save this child key now"
          variant="light"
        >
          <UiStack gap="xs">
            <UiText fw={700} size="sm">
              {secretPreview.granteeId}
            </UiText>
            <UiText size="sm">Grant: {secretPreview.grantId}</UiText>
            <UiText size="sm">Prefix: {secretPreview.keyPrefix}</UiText>
            <UiText size="sm">
              {secretPreview.secretValue
                ? `Secret: ${secretPreview.secretValue}`
                : "The control-plane response did not include a plaintext secret."}
            </UiText>
            <UiInline justify="flex-end">
              <UiButton
                onClick={() => setSecretPreview(null)}
                size="xs"
                variant="light"
              >
                Return to child keys
              </UiButton>
            </UiInline>
          </UiStack>
        </UiAlert>
      ) : null}
      <UiSurface padding="lg" radius="md" shadow="sm">
        <UiInline justify="space-between" mb="md">
          <UiInline>
            <UiText fw={700}>Child keys</UiText>
            <UiChip color="blue" variant="light">
              {ownerSummaries.length} owner accounts
            </UiChip>
          </UiInline>
          <UiButton
            disabled={snapshots.length === 0}
            onClick={() => (formOpen ? resetForm() : openForm())}
            size="sm"
            variant={formOpen ? "light" : "filled"}
          >
            {formOpen ? "Hide form" : "Create child key"}
          </UiButton>
        </UiInline>
        {formOpen ? (
          <OpeningGrantForm
            errors={formErrors}
            formState={formState}
            isSubmitting={isSubmitting}
            onCreate={onCreate}
            selectedOwnerActiveCount={selectedOwnerActiveCount}
            snapshots={snapshots}
            updateField={updateField}
          />
        ) : snapshots.length === 0 ? (
          <UiText c="dimmed" size="sm">
            No config snapshots are available for child key issuance.
          </UiText>
        ) : (
          <UiStack gap="xs">
            <UiText c="dimmed" size="sm">
              Child keys are issued per owner account and draw down that owner
              account budget.
            </UiText>
            {ownerSummaries.length > 0 ? (
              <UiInline>
                {ownerSummaries.map((summary) => (
                  <UiChip
                    color={summary.activeCount >= 8 ? "red" : "blue"}
                    key={summary.ownerAccountId}
                    variant="light"
                  >
                    {summary.ownerAccountId}: {summary.activeCount}/8 active
                  </UiChip>
                ))}
              </UiInline>
            ) : null}
          </UiStack>
        )}
      </UiSurface>
      <OpeningGrantInventory
        grants={grants}
        onRevoke={onRevoke}
        revokingGrantId={revokingGrantId}
      />
    </>
  );
}

function OpeningGrantForm({
  errors,
  formState,
  isSubmitting,
  onCreate,
  selectedOwnerActiveCount,
  snapshots,
  updateField,
}: {
  errors: OpeningGrantFormErrors;
  formState: OpeningGrantFormState;
  isSubmitting: boolean;
  onCreate: () => Promise<void>;
  selectedOwnerActiveCount: number | null;
  snapshots: ConfigSnapshotView[];
  updateField: <K extends keyof OpeningGrantFormState>(
    key: K,
    value: OpeningGrantFormState[K],
  ) => void;
}) {
  return (
    <UiStack>
      <UiSelect
        data={snapshots.map((snapshot) => ({
          label: `${snapshot.configSnapshotId} (${snapshot.status}, rev ${snapshot.revision})`,
          value: snapshot.configSnapshotId,
        }))}
        label="Config snapshot"
        onChange={(value) => updateField("configSnapshotId", value ?? "")}
        value={formState.configSnapshotId}
      />
      <FieldErrorText error={errors.configSnapshotId} />
      <UiTextField
        label="Owner account ID"
        onChange={(event) =>
          updateField("ownerAccountId", event.currentTarget.value)
        }
        placeholder="acct_acme_owner"
        value={formState.ownerAccountId}
      />
      <FieldErrorText error={errors.ownerAccountId} />
      {selectedOwnerActiveCount !== null ? (
        <UiText c={selectedOwnerActiveCount >= 8 ? "red" : "dimmed"} size="sm">
          Active child keys for this owner and project:{" "}
          {selectedOwnerActiveCount}/8
        </UiText>
      ) : null}
      <UiSelect
        data={[
          { label: "User", value: "user" },
          { label: "Workspace", value: "workspace" },
          { label: "Agent", value: "agent" },
        ]}
        label="Grantee kind"
        onChange={(value) =>
          updateField(
            "granteeKind",
            (value ?? "user") as OpeningGrantFormState["granteeKind"],
          )
        }
        value={formState.granteeKind}
      />
      <FieldErrorText error={errors.granteeKind} />
      <UiTextField
        label="Grantee ID"
        onChange={(event) => updateField("granteeId", event.currentTarget.value)}
        placeholder="user_store_001"
        value={formState.granteeId}
      />
      <FieldErrorText error={errors.granteeId} />
      <UiTextField
        label="Grantee label"
        onChange={(event) =>
          updateField("granteeLabel", event.currentTarget.value)
        }
        placeholder="Store operator 001"
        value={formState.granteeLabel}
      />
      <FieldErrorText error={errors.granteeLabel} />
      <UiTextField
        label="Expires at"
        onChange={(event) => updateField("expiresAt", event.currentTarget.value)}
        placeholder="2026-06-01T00:00:00Z"
        value={formState.expiresAt}
      />
      <FieldErrorText error={errors.expiresAt} />
      <UiTextField
        label="Scopes"
        onChange={(event) => updateField("scopes", event.currentTarget.value)}
        placeholder="route:codex,provider:hugerouter-commercial"
        value={formState.scopes}
      />
      <FieldErrorText error={errors.scopes} />
      <UiInline justify="flex-end">
        <UiButton loading={isSubmitting} onClick={() => void onCreate()}>
          Save child key
        </UiButton>
      </UiInline>
    </UiStack>
  );
}

function OpeningGrantInventory({
  grants,
  onRevoke,
  revokingGrantId,
}: {
  grants: OpeningGrantView[];
  onRevoke: (grantId: string, version: number) => Promise<void>;
  revokingGrantId: string | null;
}) {
  return (
    <UiSurface padding="lg" radius="md" shadow="sm">
      <UiInline justify="space-between" mb="md">
        <UiText fw={700}>Child key inventory</UiText>
        <UiChip color="blue" variant="light">
          {grants.length} grants
        </UiChip>
      </UiInline>
      {grants.length === 0 ? (
        <EmptyCollectionState
          description="No child keys are available for this tenant."
          title="No child keys"
        />
      ) : (
        <UiDataTable striped withTableBorder>
          <UiDataTable.Thead>
            <UiDataTable.Tr>
              <UiDataTable.Th>Owner account</UiDataTable.Th>
              <UiDataTable.Th>Grantee</UiDataTable.Th>
              <UiDataTable.Th>Key prefix</UiDataTable.Th>
              <UiDataTable.Th>Project</UiDataTable.Th>
              <UiDataTable.Th>Scopes</UiDataTable.Th>
              <UiDataTable.Th>Status</UiDataTable.Th>
              <UiDataTable.Th>Expires</UiDataTable.Th>
              <UiDataTable.Th>Actions</UiDataTable.Th>
            </UiDataTable.Tr>
          </UiDataTable.Thead>
          <UiDataTable.Tbody>
            {grants.map((grant) => (
              <UiDataTable.Tr key={grant.grantId}>
                <UiDataTable.Td>{grant.ownerAccountId}</UiDataTable.Td>
                <UiDataTable.Td>
                  {grant.granteeLabel
                    ? `${grant.granteeLabel} (${grant.granteeId})`
                    : grant.granteeId}
                </UiDataTable.Td>
                <UiDataTable.Td>{grant.credentialKeyPrefix}</UiDataTable.Td>
                <UiDataTable.Td>{grant.projectId}</UiDataTable.Td>
                <UiDataTable.Td>{grant.scopes.join(", ")}</UiDataTable.Td>
                <UiDataTable.Td>
                  <UiChip color={grant.isActive ? "teal" : "gray"} variant="light">
                    {grant.isActive ? "Active" : grant.status}
                  </UiChip>
                </UiDataTable.Td>
                <UiDataTable.Td>{grant.expiresAt}</UiDataTable.Td>
                <UiDataTable.Td>
                  <UiButton
                    disabled={
                      !grant.canRevoke ||
                      !grant.isActive ||
                      revokingGrantId === grant.grantId
                    }
                    loading={revokingGrantId === grant.grantId}
                    onClick={() => void onRevoke(grant.grantId, grant.version)}
                    size="sm"
                    variant="light"
                  >
                    Revoke
                  </UiButton>
                </UiDataTable.Td>
              </UiDataTable.Tr>
            ))}
          </UiDataTable.Tbody>
        </UiDataTable>
      )}
    </UiSurface>
  );
}
