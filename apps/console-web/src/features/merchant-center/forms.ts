export type ShopFormState = {
  merchantShopId: string;
  slug: string;
  displayName: string;
  announcement: string;
};

export type CardProductFormState = {
  cardProductId: string;
  merchantShopId: string;
  projectId: string;
  title: string;
  description: string;
  inventoryCount: string;
  faceValueUsd: string;
  retailPriceUsd: string;
  retailPriceCnyTotal: string;
  saleEnabled: boolean;
  deliveryIds: string;
  supportsTrial: string;
};

export type TrialConnectionFormState = {
  trialConnectionId: string;
  providerLabel: string;
  endpointBaseUrl: string;
  apiKey: string;
  targetModel: string;
  notes: string;
};

export type EvaluationFormState = {
  trialConnectionId: string;
};

export type FormErrors = Record<string, string | undefined>;

export function createShopForm(): ShopFormState {
  return {
    announcement: "",
    displayName: "",
    merchantShopId: "",
    slug: "",
  };
}

export function createCardProductForm(): CardProductFormState {
  return {
    cardProductId: "",
    description: "",
    deliveryIds: "",
    faceValueUsd: "",
    inventoryCount: "10",
    merchantShopId: "",
    projectId: "",
    retailPriceCnyTotal: "",
    retailPriceUsd: "",
    saleEnabled: false,
    supportsTrial: "true",
    title: "",
  };
}

export function createTrialConnectionForm(): TrialConnectionFormState {
  return {
    apiKey: "",
    endpointBaseUrl: "",
    notes: "",
    providerLabel: "",
    targetModel: "",
    trialConnectionId: "",
  };
}

export function createEvaluationForm(): EvaluationFormState {
  return {
    trialConnectionId: "",
  };
}

export function validateShopForm(form: ShopFormState): FormErrors {
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

export function validateCardProductForm(
  form: CardProductFormState,
): FormErrors {
  const errors: FormErrors = {};

  if (!/^cardprod_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.cardProductId)) {
    errors.cardProductId = "Use an id with the cardprod_ prefix.";
  }

  if (!form.merchantShopId) {
    errors.merchantShopId = "Select a merchant shop.";
  }

  if (
    form.projectId.trim() &&
    !/^proj_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(form.projectId.trim())
  ) {
    errors.projectId = "Use an id with the proj_ prefix.";
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

  if (
    form.retailPriceCnyTotal.trim() &&
    !/^\d+$/.test(form.retailPriceCnyTotal.trim())
  ) {
    errors.retailPriceCnyTotal = "Enter a CNY cent amount.";
  }

  if (!/^\d+$/.test(form.inventoryCount)) {
    errors.inventoryCount = "Enter a non-negative inventory count.";
  }

  const deliveryIds = form.deliveryIds
    .split(/[\s,]+/)
    .map((value) => value.trim())
    .filter(Boolean);
  const invalidDeliveryId = deliveryIds.find(
    (deliveryId) => !/^delivery_[A-Za-z0-9][A-Za-z0-9_-]*$/.test(deliveryId),
  );
  if (invalidDeliveryId) {
    errors.deliveryIds = "Use delivery_ ids separated by commas or new lines.";
  }

  return errors;
}

export function validateTrialConnectionForm(
  form: TrialConnectionFormState,
): FormErrors {
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
