import { Stack } from "@mantine/core";
import { useEffect, useState } from "react";
import { PageHeader } from "@huge-router/ui-kit";
import type { RouteDataResult } from "../control-plane/loaders";
import {
  getConsoleDataService,
  getControlPlaneActionErrorMessage,
} from "../control-plane/service";
import type {
  MerchantWorkspaceData,
  ReplayCapsuleView,
} from "../control-plane/types";
import { RouteErrorState } from "../control-plane/route-state";
import { MerchantCenterIntro, ShopAndTrialCards } from "./MerchantCenterForms";
import {
  CardProductCard,
  RelayEvaluationCard,
} from "./MerchantCenterProductForms";
import {
  CardProductTable,
  MerchantShopTable,
  RelayEvaluationTable,
  ReplayCapsuleDetailCard,
  TradeOrderTable,
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

export function MerchantCenterPage({
  result,
}: {
  result: MerchantCenterLoaderResult;
}) {
  if (!result || result.state === "error") {
    return (
      <Stack>
        <PageHeader
          description="Trusted AI resource library for account supply, relay quality evidence, and disclosure-first operator workflows."
          title="ku0.com - Ku"
        />
        <RouteErrorState
          kind={result?.kind}
          message={result?.state === "error" ? result.message : undefined}
          description="Resource-library data could not be loaded from the control-plane service."
          title="Library workspace unavailable"
        />
      </Stack>
    );
  }

  const [workspace, setWorkspace] = useState<MerchantWorkspaceData>(
    result.data,
  );
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
      setStatusSuccess(`Created vendor profile ${created.displayName}.`);
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
        escrowMode: cardForm.escrowMode,
        evidenceRequirement: cardForm.evidenceRequirement.trim(),
        faceValueUsd: cardForm.faceValueUsd.trim(),
        inventoryCount: Number(cardForm.inventoryCount),
        merchantShopId: cardForm.merchantShopId,
        requiredKycLevel: cardForm.requiredKycLevel,
        retailPriceUsd: cardForm.retailPriceUsd.trim(),
        resourceType: cardForm.resourceType,
        riskTier: cardForm.riskTier,
        supportsTrial: cardForm.supportsTrial === "true",
        title: cardForm.title.trim(),
      });
      setStatusSuccess(`Created account listing ${created.title}.`);
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
      setStatusSuccess(`Connected relay source ${created.providerLabel}.`);
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
        getControlPlaneActionErrorMessage(error, "relay-evaluation-create"),
      );
    } finally {
      setIsLoadingReplayCapsule(false);
    }
  }

  return (
    <Stack>
      <PageHeader
        description="Operate a trusted AI resource library with account listings, relay checks, protected orders, and disclosure-ready evidence."
        title="ku0.com - Ku"
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
      <MerchantShopTable shops={workspace.shops} />
      <CardProductTable products={workspace.cardProducts} />
      <TradeOrderTable orders={workspace.recentOrders} />
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
