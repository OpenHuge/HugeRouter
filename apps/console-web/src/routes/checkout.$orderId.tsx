import {
  UiAlert,
  UiButton,
  UiChip,
  UiHeading,
  UiInline,
  UiSegmented,
  UiStack,
  UiSurface,
  UiText,
} from "@huge-router/ui-kit";
import { createFileRoute, useNavigate } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { ensureAuthenticatedSession } from "../features/auth/auth-routing";
import { getConsoleDataService } from "../features/control-plane/service";
import type {
  MerchantProductOrderView,
  MerchantProductPrepayResult,
} from "../features/control-plane/types";

declare global {
  interface Window {
    WeixinJSBridge?: {
      invoke: (
        name: "getBrandWCPayRequest",
        params: Record<string, string>,
        callback: (response: { err_msg?: string }) => void,
      ) => void;
    };
  }
}

export const Route = createFileRoute("/checkout/$orderId")({
  beforeLoad: async ({ location }) => {
    await ensureAuthenticatedSession(location.href);
  },
  component: CheckoutRoute,
  loader: async ({ params }) =>
    getConsoleDataService().getMerchantProductOrder(params.orderId),
});

function formatCny(cents: number) {
  return `¥${(cents / 100).toFixed(2)}`;
}

function CheckoutRoute() {
  const initialOrder = Route.useLoaderData() as MerchantProductOrderView;
  const navigate = useNavigate();
  const [order, setOrder] = useState(initialOrder);
  const [channel, setChannel] = useState<"native" | "jsapi">("native");
  const [prepay, setPrepay] = useState<MerchantProductPrepayResult | null>(
    null,
  );
  const [isCreatingPrepay, setIsCreatingPrepay] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (order.status === "fulfilled" && order.pickupToken) {
      void navigate({
        params: { pickupToken: order.pickupToken },
        to: "/pickup/$pickupToken",
      });
    }
  }, [navigate, order.pickupToken, order.status]);

  const refreshOrder = useCallback(async (refreshPayment: boolean) => {
    try {
      const outTradeNo = prepay?.outTradeNo ?? order.outTradeNo;
      if (refreshPayment && outTradeNo) {
        await getConsoleDataService().getWechatPaymentOrder(outTradeNo, true);
      }
      const refreshed = await getConsoleDataService().getMerchantProductOrder(
        order.orderId,
      );
      setOrder(refreshed);
    } catch (refreshError) {
      setError(
        refreshError instanceof Error
          ? refreshError.message
          : "Unable to refresh payment status.",
      );
    }
  }, [order.orderId, order.outTradeNo, prepay?.outTradeNo]);

  useEffect(() => {
    if (!prepay?.outTradeNo && !order.outTradeNo) {
      return;
    }

    const timer = window.setInterval(() => {
      void refreshOrder(true);
    }, 3000);

    return () => window.clearInterval(timer);
  }, [order.outTradeNo, prepay?.outTradeNo, refreshOrder]);

  async function createPrepay() {
    setIsCreatingPrepay(true);
    setError(null);
    try {
      const created =
        await getConsoleDataService().createMerchantProductOrderWechatPrepay(
          order.orderId,
          channel,
        );
      setPrepay(created);
      await refreshOrder(false);
    } catch (prepayError) {
      setError(
        prepayError instanceof Error
          ? prepayError.message
          : "Unable to create WeChat Pay prepay order.",
      );
    } finally {
      setIsCreatingPrepay(false);
    }
  }

  function invokeJsapi() {
    if (!prepay?.jsapiParams) {
      setError("JSAPI parameters are not available for this order.");
      return;
    }

    if (!window.WeixinJSBridge) {
      setError("Open this checkout inside WeChat to use JSAPI payment.");
      return;
    }

    window.WeixinJSBridge.invoke(
      "getBrandWCPayRequest",
      prepay.jsapiParams,
      () => void refreshOrder(true),
    );
  }

  return (
    <UiStack maw={880} mx="auto" p="xl">
      <UiInline justify="space-between">
        <UiHeading order={1}>Checkout</UiHeading>
        <UiChip color={order.status === "fulfilled" ? "teal" : "yellow"} variant="light">
          {order.status}
        </UiChip>
      </UiInline>

      {error ? (
        <UiAlert color="red" variant="light">
          {error}
        </UiAlert>
      ) : null}

      <UiSurface padding="lg" radius={8} shadow="sm">
        <UiStack>
          <UiInline justify="space-between">
            <UiStack gap="xs">
              <UiText fw={700}>{order.orderId}</UiText>
              <UiText c="dimmed" size="sm">
                {order.cardProductId}
              </UiText>
            </UiStack>
            <UiText fw={700} size="lg">
              {formatCny(order.amountTotal)}
            </UiText>
          </UiInline>

          <UiSegmented
            data={[
              { label: "Native QR", value: "native" },
              { label: "JSAPI", value: "jsapi" },
            ]}
            onChange={setChannel}
            value={channel}
          />

          <UiInline>
            <UiButton loading={isCreatingPrepay} onClick={() => void createPrepay()}>
              Create WeChat Pay order
            </UiButton>
            <UiButton variant="light" onClick={() => void refreshOrder(true)}>
              Refresh status
            </UiButton>
          </UiInline>

          {prepay?.codeQrSvg ? (
            <UiStack gap="xs">
              <UiText fw={600}>Scan with WeChat</UiText>
              <div
                style={{ width: 220 }}
                dangerouslySetInnerHTML={{ __html: prepay.codeQrSvg }}
              />
            </UiStack>
          ) : null}

          {prepay?.codeUrl ? (
            <UiText c="dimmed" size="sm">
              {prepay.codeUrl}
            </UiText>
          ) : null}

          {prepay?.jsapiParams ? (
            <UiInline>
              <UiButton onClick={invokeJsapi}>Pay in WeChat</UiButton>
              <UiText c="dimmed" size="sm">
                {prepay.jsapiParams.package}
              </UiText>
            </UiInline>
          ) : null}
        </UiStack>
      </UiSurface>
    </UiStack>
  );
}
