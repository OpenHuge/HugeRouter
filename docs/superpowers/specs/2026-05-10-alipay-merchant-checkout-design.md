# Alipay Merchant Checkout Design

Goal: add Alipay QR-code checkout to the existing merchant product order flow, first validated on `pay-test.ku0.com` only.

Scope:
- Reuse the existing merchant order and fulfillment model.
- Add Alipay precreate support for QR payments.
- Add Alipay notify verification and order settlement.
- Keep WeChat behavior unchanged.
- Do not touch production deployment during validation.

Backend design:
- Add an `alipay` module that signs requests, verifies responses, verifies async notify signatures, and parses `alipay.trade.precreate` / `alipay.trade.query` results.
- Add routes under `/v1/merchant-product-orders/{order_id}/alipay/prepay`, `/v1/billing/alipay/notify`, and `/v1/billing/alipay/orders/{out_trade_no}`.
- Store Alipay orders in a new `alipay_payment_orders` table, mirroring the existing WeChat payment order table shape.
- Settle merchant product orders through the existing fulfillment path when Alipay reports `TRADE_SUCCESS` or `TRADE_FINISHED`.

Frontend design:
- Extend the HugeCode purchase panel to choose WeChat or Alipay QR payment.
- Keep QR display and polling UX aligned with the current WeChat card.
- Use production-safe Chinese copy and avoid provider-specific debug language.

Validation:
- Unit-test signing, response parsing, merchant route behavior, and frontend route calls.
- Deploy only `pay-test` and verify that Alipay returns a QR code URL.
