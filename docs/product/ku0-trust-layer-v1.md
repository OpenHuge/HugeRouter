# ku0 Trust Layer Product Plan V1.0

[Back to Docs Index](../README.md)

Source analysis date: **2026-04-26**  
Repository planning update: **2026-04-27**

## 1. Product Decision

ku0.com should be positioned as **ku0 Trust Layer**, the trusted quality-inspection layer for AI resources.

The product is not primarily:

- a cheap AI account marketplace
- another generic OpenAI-compatible relay
- another OpenRouter, Portkey, LiteLLM, or Cloudflare AI Gateway clone
- an ad-ranked resource directory

The durable product claim is:

> Before buying AI accounts, API quota, relay tokens, gateways, or model-provider capacity, use ku0 to verify quality, cost, model consistency, and supplier risk. After production adoption, use ku0 to continuously monitor the supplier evidence trail.

The long-term commercial mindshare is:

> Check ku0 before procurement, test with ku0 before launch, monitor with ku0 in production, and use ku0 evidence when something breaks.

## 2. Strategic Implication For This Repository

The existing HugeRouter implementation is valuable, but its role changes.

HugeRouter should no longer be documented as a gateway-first product whose main goal is broad protocol routing. It becomes the technical substrate for ku0 Trust Layer:

- `gateway-api` provides controlled request execution, route receipts, and evidence capture.
- `control-plane-api` manages tenants, provider resources, route policies, pricing, billing, merchant/evaluation data, and replay capsules.
- `edge-probe` becomes the first health and synthetic-check worker for supplier monitoring.
- `ledger-worker` and route receipt processing provide the basis for token, cost, and incident evidence.
- `apps/console-web` becomes the internal operator console first, then the supplier/customer workspace.

The first public release should prove **trusted inspection and procurement evidence**, not broad marketplace commerce.

## 3. Current Implementation Baseline

### Shipped Or Mostly Shipped

- Rust monorepo with gateway, control plane, workers, domain contracts, schemas, and TypeScript clients.
- OpenAI-compatible, Anthropic, and Gemini gateway surfaces.
- Provider resource, route policy, config snapshot, API key, usage, billing, pricing catalog, and route receipt APIs.
- Console surfaces for providers, routes, API keys, snapshots, usage, billing, route receipts, diagnostics, and merchant relay evaluation.
- Merchant workspace data model with shops, card products, trial connections, relay evaluations, and replay capsules.
- Simulated relay evaluation contract with replayable, redacted support artifacts.
- `edge-probe` cheap health and billable synthetic probe modes.
- Postgres-backed schema bootstrap for merchant, replay, receipt, pricing, auth, and billing tables.

### Bootstrap Only

- Relay evaluation is deterministic/simulated, not yet a live black-box probe.
- Merchant Center still exposes card-secret commerce concepts, which are not the target ku0 core business.
- Edge probing checks health and latency, but does not yet perform full protocol, streaming, billing, or model-consistency tests.
- Public supplier pages, shareable reports, certification badges, risk events, and supplier responses are not yet first-class product entities.
- Console wording is gateway/operator oriented, not yet ku0 procurement-trust oriented.

### Planned For First Public Release

- Live OpenAI-compatible probe runner.
- Supplier resource profiles.
- Shareable JSON/HTML inspection reports.
- Public supplier status pages.
- Basic risk event records.
- Commercial report workflow.
- Initial ku0 Verified certification states.

## 4. Product Modules

The final ku0 product line is composed of five surfaces.

| Product             | Purpose                                                  | First-release treatment                           |
| ------------------- | -------------------------------------------------------- | ------------------------------------------------- |
| ku0 Probe           | Free inspection probe for developers and buyers          | Must ship                                         |
| ku0 Reports         | Paid quality, billing, and model-consistency reports     | Must ship a manual-assisted MVP                   |
| ku0 Monitor         | Continuous uptime, latency, protocol, and billing checks | Ship basic health plus scheduled synthetic checks |
| ku0 Verified        | Supplier certification with expiring evidence            | Ship Basic Checked and Risk Watch                 |
| ku0 Trusted Gateway | Gateway only through inspected resources                 | Keep as technical substrate and private beta      |

The final nine business modules are:

- AI resource profile library
- free open-source inspection probe
- continuous monitoring
- model-consistency detection
- metering and cost verification
- public supplier status pages
- risk event library
- ku0 Verified certification system
- Trusted Gateway

## 5. First Public Version Definition

The first launch version is **ku0 Trust Layer v0.1 Public Beta**. It is intentionally narrower than the final V1.0 product vision.

It is considered launch-ready when a real user can:

1. Register or access a workspace.
2. Add a supplier endpoint with `base_url`, masked API key, and target model.
3. Run a live OpenAI-compatible probe covering basic request, streaming, first-token latency, error classification, and token-usage capture when available.
4. Receive a report with timestamp, sample count, request summary, response metadata, latency, error codes, token usage, and risk notes.
5. Share a redacted report link.
6. View a supplier profile with current status, latest check, uptime window, P95 latency, error rate, and ku0 recommendation.
7. See whether a result is self-tested, ku0-tested, supplier-paid, simulated, or live.
8. Request a paid deep report or supplier certification review.

The first version must not claim:

- complete model-truth proof
- permanent stability
- account-sharing safety
- guaranteed supplier legitimacy
- fully automated enterprise procurement replacement

## 6. Scoring And Trust Language

ku0 should avoid a single opaque score. Public and report views should show six dimensions:

| Dimension              | Suggested weight | Meaning                                                    |
| ---------------------- | ---------------: | ---------------------------------------------------------- |
| Availability           |              20% | uptime, success rate, reachability                         |
| Protocol compatibility |              15% | SDK and API shape compatibility                            |
| Model consistency      |              20% | suspected downgrade, impersonation, or capability mismatch |
| Billing transparency   |              20% | token and balance consistency                              |
| Performance stability  |              15% | latency, streaming stability, timeout behavior             |
| Transaction risk       |              10% | entity clarity, support, disputes, compliance exposure     |

External recommendation levels:

- `A`: enterprise-procurement candidate
- `B`: production trial candidate
- `C`: development/testing candidate
- `D`: small-amount cautious use only
- `E`: not recommended
- `R`: risk watch
- `U`: insufficient sample

Every conclusion must name its sample window, sample count, and detection scope.

## 7. Governance Rules

ku0 credibility depends on operating rules that are visible in product and docs:

1. Suppliers may pay for inspection, not for conclusions.
2. Commercial cooperation must be labeled.
3. Reports must show detection time, sample count, and detection scope.
4. Historical incidents are append-only; fixes are appended rather than erasing history.
5. Model-identity checks should produce confidence levels, not absolute truth claims.
6. Account sharing, credential resale, and unofficial subscription arbitrage are downgraded and risk-labeled.
7. Negative conclusions allow supplier response and re-check.
8. Community feedback, formal inspection, and commercial certification are displayed as separate evidence classes.
9. Certifications expire and downgrade automatically.
10. ku0 does not promise any supplier is permanently stable.

## 8. Multistage Launch Plan

### Stage 0: Documentation And Product Repositioning

Duration: current documentation pass.

Deliverables:

- Publish this product plan.
- Update roadmap and docs indexes to reflect ku0 Trust Layer.
- Mark existing merchant relay evaluation as a bootstrap supplier-evidence workflow.
- Deprioritize card-secret commerce and account-resource resale in public product positioning.

Exit criteria:

- New contributors can tell that the first public release is a trust/inspection product, not a generic gateway or cheap-resource marketplace.

### Stage 1: Current Baseline Hardening

Target duration: 1-2 weeks.

Deliverables:

- Audit current merchant/evaluation, route receipt, pricing, billing, and edge-probe flows.
- Ensure every simulated result is labeled as `simulated`.
- Add or tighten tests around merchant workspace creation, replay capsule access, route receipt diagnostics, pricing simulation, and probe event publishing.
- Add a seed/demo dataset for supplier profile and evaluation examples.
- Rename or wrap UI copy where needed from "merchant marketplace" toward "supplier evidence" without breaking existing routes.

Exit criteria:

- Internal operators can run the local stack and exercise the existing supplier/evaluation flow end to end.
- No UI or API surface presents simulated evaluation as live evidence.

### Stage 2: Live ku0 Probe MVP

Target duration: weeks 3-5.

Deliverables:

- Add live probe request entities and result entities to domain contracts and schemas.
- Implement OpenAI-compatible probe runner for:
  - base URL reachability
  - API key validation
  - model list or configured model check where available
  - non-streaming chat request
  - streaming request and first-token latency
  - error code normalization
  - token-usage capture when returned by provider
- Store redacted request/response summaries and hashes.
- Produce JSON report output.
- Add console action to run live probe from a trial/supplier connection.

Exit criteria:

- A developer can run a real endpoint inspection without editing configuration files.
- Reports distinguish pass, warning, fail, skipped, and unsupported checks.

### Stage 3: Supplier Profiles And Shareable Reports

Target duration: weeks 6-8.

Deliverables:

- Promote supplier resource profile as a first-class product object.
- Add public-safe report slugs and redaction policy.
- Add HTML report view.
- Add supplier status summary:
  - current status
  - latest check time
  - 24-hour and 7-day availability
  - P50/P95 latency
  - error rate
  - streaming stability
  - latest risk notes
- Add supplier response field and internal moderation state.

Exit criteria:

- ku0 can be used as a procurement-precheck link before buying a supplier resource.
- Shared reports do not expose raw API keys, raw prompts, or sensitive payloads.

### Stage 4: Reports And Certification Beta

Target duration: weeks 9-12.

Deliverables:

- Add report order/intake workflow for:
  - single-supplier deep report
  - supplier comparison report
  - billing anomaly report
  - model-consistency report
  - launch-readiness report
- Add certification states:
  - `ku0 Basic Checked`
  - `ku0 Billing Transparent`
  - `ku0 Risk Watch`
  - `ku0 Not Recommended`
- Add expiry dates and re-check triggers.
- Add supplier-paid, ku0-self-tested, and customer-submitted evidence labels.
- Add manual operator workflow for approving public report publication.

Exit criteria:

- The team can charge for a manually reviewed report while the automated probe covers objective evidence.
- Supplier certification is evidence-backed, expiring, and appealable.

### Stage 5: Public Beta Launch

Target duration: launch week after Stage 4.

Deliverables:

- Public homepage copy:
  - "AI resource procurement starts with ku0 inspection."
  - "Check model identity, protocol compatibility, token metering, real cost, stability, and supplier risk."
- Public supplier directory with limited curated entries.
- Public report examples.
- Clear legal and risk disclaimers.
- Support workflow for supplier response and re-check requests.
- Launch metrics dashboard.

Exit criteria:

- At least 20 supplier profiles are present.
- At least 5 real live probe reports are publishable.
- At least 2 suppliers or buyers complete a paid/report-intent workflow.
- Public pages consistently separate inspection evidence from ads or cooperation.

## 9. Post-Launch Roadmap

### 3-6 Months

- Scheduled continuous monitoring.
- Risk event library.
- Supplier appeal and re-check workflow.
- ku0 Verified complete certification ladder.
- Monthly AI resource quality report.
- Billing transparency test v1.
- Model consistency test v1.
- Enterprise alerting and procurement whitelist export.

### 6-12 Months

- Trusted Gateway private beta.
- Production traffic quality feedback into supplier scores.
- Enterprise supplier whitelist and audit workflow.
- Private synthetic test suites.
- Procurement consulting package.
- Industry benchmark reports.
- Private deployment option for enterprise monitoring.

## 10. North Star And KPIs

North star:

> Monthly AI resource spend inspected or monitored by ku0.

Primary KPIs:

- live probe runs per day
- new supplier profiles per week
- shared report views
- report re-check rate
- supplier response rate
- paid report conversion
- certification applications
- monitored suppliers
- historical risk events
- enterprise procurement leads

Quality KPIs:

- false-positive correction rate
- sample coverage per public conclusion
- report publication review time
- expired certification downgrade rate
- incident-to-recheck latency

## 11. Engineering Ownership Map

| Workstream               | Primary paths                                                                            |
| ------------------------ | ---------------------------------------------------------------------------------------- |
| Domain and schemas       | `crates/core-domain`, `schemas/*`, `packages/ts-shared-schema`, `packages/ts-api-client` |
| Control plane            | `services/control-plane-api`                                                             |
| Gateway evidence capture | `services/gateway-api`, `crates/provider-*`, `crates/protocol-*`                         |
| Probing and monitoring   | `services/edge-probe`, worker services, event schemas                                    |
| Metering and reports     | `crates/metering`, `services/ledger-worker`, `services/route-receipt-worker`             |
| Console and public views | `apps/console-web`, `packages/ui-kit`                                                    |
| Operations               | `infra/*`, `.github/*`, `justfile`, runbooks                                             |

## 12. First Backlog Epics

1. Reposition Merchant Center into Supplier Evidence Center.
2. Add live probe contracts and result persistence.
3. Implement OpenAI-compatible live probe runner.
4. Add report generation and shareable redacted report view.
5. Add supplier profile and public status summary.
6. Add risk event and supplier response model.
7. Add certification state, expiry, and evidence labels.
8. Add launch metrics and operator moderation workflow.

## 13. Stage-To-Engineering Plan

| Stage                             | Backend and contracts                                                                          | Frontend and product surface                                             | Data and ops                                                         | Verification                                                                                         |
| --------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | -------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Stage 1 baseline hardening        | tighten merchant evaluation, replay capsule, route receipt, pricing, and probe event contracts | relabel simulated evaluation and supplier evidence surfaces              | seed demo supplier/evaluation data                                   | unit and HTTP tests for existing control-plane flows                                                 |
| Stage 2 live Probe MVP            | add `probe_run`, `probe_check`, `probe_report` contracts and OpenAI-compatible live runner     | add run-probe action and JSON result view                                | store redacted summaries, hashes, timings, and normalized errors     | mock upstream integration tests for non-streaming, streaming, auth failure, timeout, and token usage |
| Stage 3 supplier profiles         | add supplier profile, public report slug, status summary, supplier response, moderation state  | add supplier profile page and HTML report page                           | aggregate availability, latency, error rate, and streaming stability | snapshot tests for public-safe report rendering and redaction                                        |
| Stage 4 reports and certification | add report order, report type, certification state, expiry, and evidence source contracts      | add operator review, paid report intake, and certification state display | add re-check triggers and certification downgrade job                | tests for expiry, re-check, evidence labeling, and moderation workflow                               |
| Stage 5 public beta               | freeze public API/report contracts and launch copy                                             | publish homepage, supplier directory, examples, and support flow         | add launch dashboard and incident/re-check runbook                   | release checklist, smoke tests, link checks, and production dry run                                  |

Critical path:

1. Contracts and schemas must land before backend persistence and UI consumers.
2. Live probe runner must land before public reports can claim live evidence.
3. Redaction policy must land before any shareable report or supplier page is public.
4. Supplier profile aggregation must land before certification states are exposed.
5. Certification expiry and evidence-source labels must land before supplier-paid inspection is accepted.

First-version release gate:

- `git diff --check` passes.
- Rust workspace check and relevant service tests pass.
- Frontend typecheck and relevant console tests pass.
- At least one seeded supplier can be inspected end to end in local stack.
- Public report fixture contains no raw API key, raw prompt, or unredacted sensitive payload.
- Product pages state evidence source, sample window, sample count, and detection scope.
