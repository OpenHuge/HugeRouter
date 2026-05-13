import {
  UiAlert,
  UiButton,
  UiChip,
  UiGrid,
  UiHeading,
  UiInline,
  UiStack,
  UiSurface,
  UiText,
} from "@huge-router/ui-kit";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useState } from "react";
import { getConsoleDataService } from "../features/control-plane/service";
import type { MerchantPublicShopView } from "../features/control-plane/types";
import { useRequestContext } from "../start";

export const Route = createFileRoute("/shops/$slug")({
  component: ShopRoute,
  loader: async ({ params }) => getConsoleDataService().getPublicShop(params.slug),
});

function formatCny(cents?: number) {
  return cents == null ? "Not priced" : `¥${(cents / 100).toFixed(2)}`;
}

function ShopRoute() {
  const shop = Route.useLoaderData() as MerchantPublicShopView;
  const navigate = useNavigate();
  const requestContext = useRequestContext();
  const [creatingProductId, setCreatingProductId] = useState<string | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const currentPath =
    typeof window === "undefined" ? `/shops/${shop.shop.slug}` : window.location.pathname;

  async function startCheckout(cardProductId: string) {
    if (requestContext.session.kind !== "authenticated") {
      await navigate({
        search: { redirect: currentPath } as never,
        to: "/login",
      });
      return;
    }

    setCreatingProductId(cardProductId);
    setError(null);
    try {
      const order =
        await getConsoleDataService().createMerchantProductOrder(cardProductId);
      await navigate({
        params: { orderId: order.orderId },
        to: "/checkout/$orderId",
      });
    } catch (checkoutError) {
      setError(
        checkoutError instanceof Error
          ? checkoutError.message
          : "Unable to create checkout order.",
      );
    } finally {
      setCreatingProductId(null);
    }
  }

  return (
    <UiStack maw={1120} mx="auto" p="xl">
      <UiInline align="flex-start" justify="space-between">
        <UiStack gap="xs">
          <UiHeading order={1}>{shop.shop.displayName}</UiHeading>
          {shop.shop.announcement ? (
            <UiText c="dimmed">{shop.shop.announcement}</UiText>
          ) : null}
        </UiStack>
        <UiChip color="teal" variant="light">
          WeChat Pay
        </UiChip>
      </UiInline>

      {error ? (
        <UiAlert color="red" variant="light">
          {error}
        </UiAlert>
      ) : null}

      <UiGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="md">
        {shop.products.map(({ availableInventoryCount, product }) => {
          const inStock = availableInventoryCount > 0;

          return (
            <UiSurface key={product.cardProductId} padding="lg" radius={8} shadow="sm">
              <UiStack>
                <UiInline justify="space-between">
                  <UiText fw={700}>{product.title}</UiText>
                  <UiChip color={inStock ? "teal" : "gray"} variant="light">
                    {inStock ? `${availableInventoryCount} in stock` : "Sold out"}
                  </UiChip>
                </UiInline>
                <UiText c="dimmed" size="sm">
                  {product.description}
                </UiText>
                <UiText fw={700} size="lg">
                  {formatCny(product.retailPriceCnyTotal)}
                </UiText>
                <UiButton
                  disabled={!inStock}
                  loading={creatingProductId === product.cardProductId}
                  onClick={() => void startCheckout(product.cardProductId)}
                >
                  Buy
                </UiButton>
              </UiStack>
            </UiSurface>
          );
        })}
      </UiGrid>

      {shop.products.length === 0 ? (
        <UiAlert color="orange" variant="light">
          This shop has no sale-enabled card products.
        </UiAlert>
      ) : null}
    </UiStack>
  );
}
