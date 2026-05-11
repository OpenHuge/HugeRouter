# Guest Payment Orders Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add guest order binding and history lookup to HugeRouter merchant product checkout.

**Architecture:** Store guest lookup metadata inside existing `MerchantProductOrderRecord` JSON payload to avoid schema churn. Add a public history endpoint that hashes normalized phone plus passphrase and returns public order views.

**Tech Stack:** Rust, Axum, Serde, existing memory/Postgres store abstraction.

---

## Tasks

- [ ] Add failing merchant checkout tests for generated lookup passphrase and history lookup.
- [ ] Extend merchant order record/public view with optional buyer contact metadata.
- [ ] Add normalization, passphrase generation, and lookup hashing in `merchant_checkout_api.rs`.
- [ ] Persist guest metadata in memory and Postgres order creation paths.
- [ ] Add store methods for history lookup across memory/Postgres.
- [ ] Register `POST /v1/merchant-product-orders/history` in `lib.rs`.
- [ ] Run targeted control-plane tests and commit.
