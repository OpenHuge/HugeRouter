#!/usr/bin/env node

import { readFileSync } from "node:fs";
import process from "node:process";

function readArgs(argv) {
  const args = new Map();
  for (let index = 2; index < argv.length; index += 1) {
    const key = argv[index];
    if (key === "--") {
      continue;
    }
    const value = argv[index + 1];
    if (key === "--help") {
      args.set("help", "true");
      continue;
    }
    if (!key.startsWith("--") || value == null || value.startsWith("--")) {
      throw new Error(`Invalid argument near ${key}`);
    }
    args.set(key.slice(2), value);
    index += 1;
  }
  return args;
}

function envName(key) {
  return `SEED_${key.toUpperCase().replaceAll("-", "_")}`;
}

function required(args, key) {
  const value = args.get(key) ?? process.env[envName(key)];
  if (!value || value.trim().length === 0) {
    throw new Error(`Missing --${key}`);
  }
  return value.trim();
}

function optional(args, key, fallback) {
  const value = args.get(key) ?? process.env[envName(key)];
  return value && value.trim().length > 0 ? value.trim() : fallback;
}

function optionalNullable(args, key) {
  const value = optional(args, key, "");
  return value.length > 0 ? value : null;
}

function loadDeliveryIds(args) {
  const inline = optional(args, "delivery-ids", "");
  if (inline) {
    return inline
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean);
  }
  const filePath = optional(args, "delivery-ids-file", "");
  if (!filePath) {
    return [];
  }
  return readFileSync(filePath, "utf8")
    .split(/\r?\n/)
    .map((value) => value.trim())
    .filter(Boolean);
}

async function requestJson(baseUrl, path, token, body) {
  const response = await fetch(new URL(path, baseUrl), {
    body: JSON.stringify(body),
    headers: {
      Accept: "application/json",
      Authorization: `Bearer ${token}`,
      "Content-Type": "application/json",
    },
    method: "POST",
  });
  const text = await response.text();
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}: ${text}`);
  }
  return text ? JSON.parse(text) : null;
}

function printHelp() {
  console.log(`Usage:
  pnpm seed:hugecode-shop -- --base-url https://router.example --token TOKEN \\
    --merchant-shop-id mshop_hugecode --shop-slug hugecode-cdk \\
    --display-name "HugeCode 交付中心" --card-product-id cprod_hugecode_pro_week \\
    --title "ChatGPT Pro 周租" --description "付款后领取交付凭证" \\
    --inventory-count 3 --retail-price-cny-total 9900 --delivery-ids delivery_1,delivery_2,delivery_3

Required:
  --base-url
  --token
  --merchant-shop-id
  --shop-slug
  --display-name
  --card-product-id
  --title
  --description
  --inventory-count
  --retail-price-cny-total

Optional:
  --announcement
  --project-id
  --face-value-usd
  --retail-price-usd
  --delivery-ids
  --delivery-ids-file
`);
}

async function main() {
  const args = readArgs(process.argv);
  if (args.has("help")) {
    printHelp();
    return;
  }

  const baseUrl = required(args, "base-url");
  const token = required(args, "token");
  const merchantShopId = required(args, "merchant-shop-id");
  const shopSlug = required(args, "shop-slug");
  const displayName = required(args, "display-name");
  const cardProductId = required(args, "card-product-id");
  const title = required(args, "title");
  const description = required(args, "description");
  const inventoryCount = Number.parseInt(required(args, "inventory-count"), 10);
  const retailPriceCnyTotal = Number.parseInt(required(args, "retail-price-cny-total"), 10);
  const deliveryIds = loadDeliveryIds(args);

  if (!Number.isInteger(inventoryCount) || inventoryCount < 0) {
    throw new Error("--inventory-count must be a non-negative integer");
  }
  if (!Number.isInteger(retailPriceCnyTotal) || retailPriceCnyTotal <= 0) {
    throw new Error("--retail-price-cny-total must be a positive integer in cents");
  }
  if (deliveryIds.length > 0 && deliveryIds.length !== inventoryCount) {
    throw new Error("delivery id count must match --inventory-count when delivery ids are provided");
  }

  const shop = await requestJson(baseUrl, "/v1/merchant/shops", token, {
    announcement: optionalNullable(args, "announcement"),
    display_name: displayName,
    merchant_shop_id: merchantShopId,
    slug: shopSlug,
  });

  const product = await requestJson(baseUrl, "/v1/merchant/card-products", token, {
    card_product_id: cardProductId,
    delivery_ids: deliveryIds,
    description,
    face_value_usd: optional(args, "face-value-usd", "0"),
    inventory_count: inventoryCount,
    merchant_shop_id: merchantShopId,
    project_id: optionalNullable(args, "project-id"),
    retail_price_cny_total: retailPriceCnyTotal,
    retail_price_usd: optional(args, "retail-price-usd", "0"),
    sale_enabled: true,
    supports_trial: false,
    title,
  });

  console.log(JSON.stringify({ product, shop }, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : error);
  process.exitCode = 1;
});
