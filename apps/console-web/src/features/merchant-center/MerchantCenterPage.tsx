import { UiStack } from "@huge-router/ui-kit";
import { useEffect, useState } from "react";
import { PageHeader } from "@huge-router/ui-kit";
import type { RouteDataResult } from "../control-plane/loaders";
import {
  getConsoleDataService,
  getControlPlaneActionErrorMessage,
} from "../control-plane/service";
import type {
  MerchantWorkspaceData,
  RelayEvaluationView,
  ReplayCapsuleView,
} from "../control-plane/types";
import { RouteErrorState } from "../control-plane/route-state";
import { MerchantCenterIntro, ShopAndTrialCards } from "./MerchantCenterForms";
import {
  CardProductCard,
  RelayEvaluationCard,
} from "./MerchantCenterProductForms";
import { MerchantEvidenceSummary } from "./MerchantEvidenceSummary";
import {
  CardProductTable,
  MerchantShopTable,
  RelayEvaluationTable,
  ReplayCapsuleDetailCard,
  TrialConnectionTable,
} from "./MerchantCenterTables";
import {
  createCardProductForm,
  createEvaluationForm,
  createShopForm,
  createTrialConnectionForm,
  validateCardProductForm,
  validateShopForm,
  validateTrialConnectionForm,
  type CardProductFormState,
  type EvaluationFormState,
  type FormErrors,
  type ShopFormState,
  type TrialConnectionFormState,
} from "./forms";

type MerchantCenterLoaderResult = RouteDataResult<MerchantWorkspaceData>;
type RelayEvaluationVerdictFilter = "all" | RelayEvaluationView["verdict"];

export function MerchantCenterPage({
  result,
}: {
  result: MerchantCenterLoaderResult;
}) {
  if (!result || result.state === "error") {
    return (
      <UiStack>
        <PageHeader
          description="Manage supplier test endpoints, replayable evaluation evidence, and legacy private inventory workflows."
          title="Supplier Evidence Center"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result.message : undefined}
          description="Supplier evidence data could not be loaded from the control-plane service."
          title="Supplier evidence unavailable"
        />
      </UiStack>
    );
  }

  return <MerchantCenterContent data={result.data} />;
}

function MerchantCenterContent({ data }: { data: MerchantWorkspaceData }) {
  const [workspace, setWorkspace] = useState<MerchantWorkspaceData>(data);
  const [evaluationVerdictFilter, setEvaluationVerdictFilter] =
    useState<RelayEvaluationVerdictFilter>("all");
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
    setWorkspace(data);
  }, [data]);

  useEffect(() => {
    setCardForm((current) => ({
      ...current,
      merchantShopId:
        current.merchantShopId || data.shops[0]?.merchantShopId || "",
    }));
    setEvaluationForm((current) => ({
      trialConnectionId:
        current.trialConnectionId ||
        data.trialConnections[0]?.trialConnectionId ||
        "",
    }));
  }, [data.shops, data.trialConnections]);

  async function refreshWorkspace() {
    setWorkspace(await getConsoleDataService().getMerchantWorkspace());
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
      const deliveryIds = cardForm.deliveryIds
        .split(/[\s,]+/)
        .map((value) => value.trim())
        .filter(Boolean);
      const created = await getConsoleDataService().createCardProduct({
        cardProductId: cardForm.cardProductId.trim(),
        description: cardForm.description.trim(),
        deliveryIds,
        faceValueUsd: cardForm.faceValueUsd.trim(),
        inventoryCount: Number(cardForm.inventoryCount),
        merchantShopId: cardForm.merchantShopId,
        projectId: cardForm.projectId.trim() || undefined,
        retailPriceCnyTotal: cardForm.retailPriceCnyTotal.trim()
          ? Number(cardForm.retailPriceCnyTotal)
          : undefined,
        retailPriceUsd: cardForm.retailPriceUsd.trim(),
        saleEnabled: cardForm.saleEnabled,
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
      setStatusError(
        "Select a trial connection before starting an evaluation.",
      );
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
        `Recorded supplier evidence ${evaluation.relayEvaluationId} with replay capsule ${evaluation.replayCapsuleId}.`,
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

  async function onViewReplayCapsule(replayCapsuleId: string) {
    setIsLoadingReplayCapsule(true);
    setSelectedReplayCapsuleId(replayCapsuleId);
    setStatusError(null);

    try {
      const replayCapsule =
        await getConsoleDataService().getReplayCapsule(replayCapsuleId);
      setSelectedReplayCapsule(replayCapsule);
    } catch (error) {
      setSelectedReplayCapsule(null);
      setStatusError(
        getControlPlaneActionErrorMessage(error, "replay-capsule-load"),
      );
    } finally {
      setIsLoadingReplayCapsule(false);
    }
  }

  return (
    <UiStack>
      <PageHeader
        description="Attach dedicated supplier test endpoints, retain redacted replay evidence, and keep legacy commerce workflows private."
        title="Supplier Evidence Center"
      />
      <MerchantCenterIntro
        statusNotice={{
          error: statusError,
          onDismiss: () => {
            setStatusError(null);
            setStatusSuccess(null);
          },
          success: statusSuccess,
        }}
      />
      <ShopAndTrialCards
        isSubmittingShop={isSubmittingShop}
        isSubmittingTrial={isSubmittingTrial}
        onCreateShop={onCreateShop}
        onCreateTrialConnection={onCreateTrialConnection}
        setShopForm={setShopForm}
        setTrialForm={setTrialForm}
        shopErrors={shopErrors}
        shopForm={shopForm}
        statusNotice={{
          error: statusError,
          onDismiss: () => {
            setStatusError(null);
            setStatusSuccess(null);
          },
          success: statusSuccess,
        }}
        trialErrors={trialErrors}
        trialForm={trialForm}
      />
      <CardProductCard
        cardErrors={cardErrors}
        cardForm={cardForm}
        isSubmittingCard={isSubmittingCard}
        onCreateCardProduct={onCreateCardProduct}
        setCardForm={setCardForm}
        shops={workspace.shops}
      />
      <RelayEvaluationCard
        evaluationForm={evaluationForm}
        isSubmittingEvaluation={isSubmittingEvaluation}
        onRunEvaluation={onRunEvaluation}
        setEvaluationForm={setEvaluationForm}
        trialConnections={workspace.trialConnections}
      />
      <MerchantEvidenceSummary workspace={workspace} />
      <MerchantShopTable shops={workspace.shops} />
      <CardProductTable products={workspace.cardProducts} />
      <TrialConnectionTable connections={workspace.trialConnections} />
      <RelayEvaluationTable
        evaluationVerdictFilter={evaluationVerdictFilter}
        isLoadingReplayCapsule={isLoadingReplayCapsule}
        onViewReplayCapsule={onViewReplayCapsule}
        setEvaluationVerdictFilter={setEvaluationVerdictFilter}
        selectedReplayCapsuleId={selectedReplayCapsuleId}
        workspace={workspace}
      />
      <ReplayCapsuleDetailCard replayCapsule={selectedReplayCapsule} />
    </UiStack>
  );
}
