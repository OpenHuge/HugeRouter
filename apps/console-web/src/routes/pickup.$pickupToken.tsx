import {
  UiAlert,
  UiButton,
  UiChip,
  UiHeading,
  UiInline,
  UiStack,
  UiSurface,
  UiText,
} from "@huge-router/ui-kit";
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import {
  getConsoleDataService,
  toControlPlaneUrl,
} from "../features/control-plane/service";
import type { MerchantPickupView } from "../features/control-plane/types";

export const Route = createFileRoute("/pickup/$pickupToken")({
  component: PickupRoute,
  loader: async ({ params }) =>
    getConsoleDataService().getMerchantPickup(params.pickupToken),
});

function PickupRoute() {
  const pickup = Route.useLoaderData() as MerchantPickupView;
  const [isDownloading, setIsDownloading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function downloadArtifact() {
    setIsDownloading(true);
    setError(null);
    try {
      const response = await fetch(toControlPlaneUrl("/v1/delivery-downloads/artifact"), {
        credentials: "include",
        headers: {
          Authorization: `Bearer ${pickup.downloadToken}`,
        },
      });

      if (!response.ok) {
        throw new Error(await response.text());
      }

      const blob = await response.blob();
      const disposition = response.headers.get("content-disposition") ?? "";
      const fileName =
        disposition.match(/filename="([^"]+)"/)?.[1] ??
        `${pickup.order.orderId}.bin`;
      const objectUrl = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = objectUrl;
      anchor.download = fileName;
      anchor.click();
      URL.revokeObjectURL(objectUrl);
    } catch (downloadError) {
      setError(
        downloadError instanceof Error
          ? downloadError.message
          : "Unable to download delivery artifact.",
      );
    } finally {
      setIsDownloading(false);
    }
  }

  return (
    <UiStack maw={760} mx="auto" p="xl">
      <UiInline justify="space-between">
        <UiHeading order={1}>Pickup</UiHeading>
        <UiChip color="teal" variant="light">
          {pickup.order.status}
        </UiChip>
      </UiInline>

      {error ? (
        <UiAlert color="red" variant="light">
          {error}
        </UiAlert>
      ) : null}

      <UiSurface padding="lg" radius={8} shadow="sm">
        <UiStack>
          <UiText fw={700}>{pickup.order.orderId}</UiText>
          <UiText c="dimmed" size="sm">
            Delivery {pickup.order.inventoryDeliveryId}
          </UiText>
          <UiInline>
            <UiButton loading={isDownloading} onClick={() => void downloadArtifact()}>
              Download
            </UiButton>
            <UiText c="dimmed" size="sm">
              Grant {pickup.order.downloadGrantId ?? "pending"}
            </UiText>
          </UiInline>
        </UiStack>
      </UiSurface>
    </UiStack>
  );
}
