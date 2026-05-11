# Alipay Merchant Checkout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Alipay QR checkout to the existing merchant product order flow and validate it on `pay-test.ku0.com`.

**Architecture:** Add an Alipay client module parallel to `wechat_pay.rs`, an API module parallel to `wechat_pay_api.rs`, and merchant checkout route support. Reuse the existing payment-order record shape and fulfillment-by-out-trade-no path to avoid adding unrelated domain changes.

**Tech Stack:** Rust, Axum, SQLx/Postgres, RSA2 signing, Alipay openapi gateway, existing HugeRouter merchant checkout store.

---

## Tasks

- Add failing tests for Alipay signing/response parsing and merchant checkout route wiring.
- Implement `services/control-plane-api/src/alipay.rs` for sign, precreate, query, notify verification.
- Implement `services/control-plane-api/src/alipay_api.rs` for generic billing routes.
- Extend store with `alipay_payment_orders` using the same persisted shape as WeChat orders.
- Extend merchant checkout with `/alipay/prepay` and settlement on Alipay paid states.
- Update `lib.rs` routes and schema.
- Deploy to `pay-test` with local `sub2apipay` Alipay credentials and verify QR generation.
