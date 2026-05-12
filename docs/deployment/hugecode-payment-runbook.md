# HugeCode Payment Deployment Runbook

## Scope

This runbook covers the first-release payment integration:

- HugeRouter owns WeChat Pay Native QR payment.
- HugeCode renders the QR code returned by HugeRouter.
- Delivery is card/file/credential inventory.
- Alipay is not part of this release.
- Balance recharge is not part of this release.

## HugeRouter Environment

Configure these on the HugeRouter server:

```dotenv
CLIENT_BROWSER_PROXY_TOKEN=
CLIENT_BROWSER_PROXY_SCHEME=socks5
CLIENT_BROWSER_PROXY_HOST=
CLIENT_BROWSER_PROXY_PORT=
CLIENT_BROWSER_PROXY_USERNAME=
CLIENT_BROWSER_PROXY_PASSWORD=
CLIENT_BROWSER_PROXY_CONNECT_HOST=
CLIENT_BROWSER_PROXY_BYPASS_RULES=<local>;localhost;127.0.0.1;::1

WECHAT_PAY_APP_ID=
WECHAT_PAY_MCH_ID=
WECHAT_PAY_MERCHANT_SERIAL_NO=
WECHAT_PAY_MERCHANT_PRIVATE_KEY_PATH=
WECHAT_PAY_MERCHANT_PRIVATE_KEY=
WECHAT_PAY_API_V3_KEY=
WECHAT_PAY_NOTIFY_URL=https://<router-domain>/v1/billing/wechat-pay/notify
WECHAT_PAY_PLATFORM_PUBLIC_KEY_PATH=
WECHAT_PAY_PLATFORM_PUBLIC_KEY=
WECHAT_PAY_MIN_AMOUNT_TOTAL=1
WECHAT_PAY_MAX_AMOUNT_TOTAL=1000000
```

Use either `WECHAT_PAY_MERCHANT_PRIVATE_KEY_PATH` or `WECHAT_PAY_MERCHANT_PRIVATE_KEY`.
Use either `WECHAT_PAY_PLATFORM_PUBLIC_KEY_PATH` or `WECHAT_PAY_PLATFORM_PUBLIC_KEY`.

`CLIENT_BROWSER_PROXY_TOKEN` is the token packaged into HugeCode as
`clientBrowserProxyToken`. The proxy host, port, username, password, and
connect host stay on the HugeRouter server and are returned only from
`GET /v1/client/browser-proxy` after `Authorization: Bearer $CLIENT_BROWSER_PROXY_TOKEN`.

## HugeCode Environment

Configure HugeCode with public routing only:

```dotenv
VITE_OPENHUGE_CONTROL_PLANE_BASE_URL=https://<router-domain>
OPENHUGE_CLIENT_BROWSER_PROXY_TOKEN=<client-browser-proxy-token>
VITE_OPENHUGE_CDK_SHOP_SLUG=hugecode-cdk
VITE_OPENHUGE_WORKSPACE_SLUG=openhuge
```

Do not put proxy host, proxy password, WeChat merchant keys, APIv3 keys, or platform public keys in HugeCode.

## WeChat Merchant Backend

Configure the payment notification URL to:

```text
https://<router-domain>/v1/billing/wechat-pay/notify
```

The domain must be HTTPS and publicly reachable by WeChat Pay.

Confirm whether the merchant account uses platform certificate mode or platform public-key mode. The current HugeRouter implementation expects configured platform public key PEM material through `WECHAT_PAY_PLATFORM_PUBLIC_KEY` or `WECHAT_PAY_PLATFORM_PUBLIC_KEY_PATH`.

## Seed Shop and Product

Prepare delivery records first. Then bind their delivery IDs to the card product.

Example:

```bash
pnpm seed:hugecode-shop -- \
  --base-url https://<router-domain> \
  --token <admin-or-merchant-token> \
  --merchant-shop-id mshop_hugecode \
  --shop-slug hugecode-cdk \
  --display-name "HugeCode 交付中心" \
  --announcement "付款后领取交付凭证" \
  --card-product-id cprod_hugecode_pro_week \
  --title "ChatGPT Pro 周租" \
  --description "付款后领取对应交付凭证" \
  --inventory-count 1 \
  --retail-price-cny-total 1 \
  --delivery-ids delivery_example
```

Use a 1-cent product only for live payment smoke tests. Change the product price before production.

## Smoke Test

1. Open HugeCode with the configured HugeRouter base URL and shop slug.
2. Confirm the product list loads.
3. Start purchase.
4. Confirm WeChat Native QR renders.
5. Scan and pay a low amount.
6. Confirm HugeRouter logs show notification verification success.
7. Confirm order status changes to `fulfilled`.
8. Confirm HugeCode pickup/download succeeds.
9. Confirm unpaid or failed orders cannot access pickup data.

## Rollback

If payment fails in production:

1. Disable `sale_enabled` for the affected product or remove available inventory.
2. Keep HugeRouter online so WeChat retry notifications can still be received.
3. Do not rotate keys unless verification failure is caused by compromised or incorrect key material.
4. Restore the previous HugeCode build if UI behavior blocks existing CDK redemption.
