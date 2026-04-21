# 23. Frontend Specification

[Back to Docs Index](../README.md)

The console is implemented with **TanStack Start** and uses **Mantine** as the shared UI foundation. The goal is to move quickly with a cohesive admin-grade component system without over-investing in custom design infrastructure before the product shape is proven.

As of **April 20, 2026**, the recommended baseline is:

- React `19.2+`
- Mantine `9.0.x`
- TanStack Start `1.x`

Repository policy:

- keep Start, Router, and tightly coupled packages pinned to exact versions in the repository even when the upstream project is on a `1.x` line
- treat the lockfile and exact version set as part of the architecture baseline, not as incidental package-manager output

### 23.1 Frontend Applications

At minimum, `apps/console-web` should provide:

- login and session management
- admin console
- tenant console
- billing dashboard
- route management UI
- provider resource management UI
- usage explorer
- audit explorer
- request diagnostics view

### 23.2 Route Structure

Suggested route groupings:

```text
/
|-- /login
|-- /app
|   |-- /overview
|   |-- /projects
|   |-- /keys
|   |-- /models
|   |-- /routes
|   |-- /providers
|   |-- /usage
|   |-- /billing
|   |-- /audit
|   |-- /settings
|   `-- /diagnostics
`-- /admin
    |-- /tenants
    |-- /plans
    |-- /system-health
    |-- /provider-health
    |-- /global-policies
    `-- /operations
```

### 23.3 Provider Stack and Application Shell

The root frontend provider stack should include:

- `MantineProvider` configured from `packages/design-tokens`
- `QueryClientProvider` for TanStack Query
- Mantine notifications
- Mantine modals
- TanStack Router error boundaries and pending UI
- `src/start.ts` for TanStack Start global middleware and request-level configuration

The authenticated shell should be built around Mantine's `AppShell` primitives plus internal wrappers from `packages/ui-kit` so that navigation, breadcrumbs, headers, and content frames are consistent across the product.

Recommended shell policy:

- use standard fixed `AppShell` layout for the authenticated admin and tenant console
- use Mantine 9 `AppShell` static mode for auth pages, setup flows, embedded documentation, or low-chrome pages where fixed positioning would be awkward
- centralize shell mode decisions in `packages/ui-kit/shell` instead of scattering them across route files

### 23.4 Frontend Data Access

Use TanStack Query for:

- caching server state
- background refetch
- optimistic updates where safe
- invalidation on resource mutation
- sharing typed resource hooks built on top of the generated API client

Route loaders and server functions should remain the first choice for route-critical data and auth-aware mutations. Query hooks should wrap the generated client rather than duplicating endpoint definitions in application code.

TanStack Start global middleware should be introduced early through `src/start.ts` so auth, tracing, and request context are not retrofitted after routes are already proliferating.

OpenAI-facing console features should be designed around Responses-era artifacts first:

- show response, tool, and event-oriented diagnostics rather than only chat-message transcripts
- model request inspection around typed items and request metadata, not only `choices[0].message`
- keep Chat Completions compatibility views as adapters at the UI edge, not as the canonical internal representation

### 23.5 Mantine Usage Model

Mantine is the default component layer for:

- layout primitives
- buttons and input controls
- tabs, drawers, modals, and overlays
- notifications and inline feedback
- typography, spacing, and theming
- admin-friendly dense forms and data entry flows

We should prefer composition over broad wrapper abstraction:

- expose Mantine directly inside app features when no shared opinion is needed
- create shared wrappers in `packages/ui-kit` only when we need product-specific defaults, access patterns, or repeated combinations
- keep custom CSS focused on layout, feature-specific visual affordances, and integration gaps rather than rebuilding a design system from scratch

### 23.6 UI Requirements

The UI must support:

- high-density tables for usage and ledger data
- advanced filtering and search
- saved views
- JSON policy editing with validation
- route simulation UX
- trace and request diagnostics viewer
- a clean theme foundation that supports dark mode later without forcing it into the first milestone

### 23.7 Forms and Validation

The primary form stack should be:

- TanStack Form for typed form state
- Zod for schema validation
- Mantine input components for rendering

Prefer Standard Schema-compatible Zod schemas at the package boundary so forms, server validation, and generated docs can converge on one representation over time.

Form primitives in `packages/ui-kit` should cover the repeated patterns we know we will need early:

- text and secret inputs
- select and multi-select controls
- JSON/textarea editors with inline validation
- destructive-action confirmations
- asynchronous submit and retry states

### 23.8 Frontend Security

- admin routes must enforce role-aware data fetching
- server functions must validate auth independently of client state
- secrets must never be rendered back after creation where reveal-once semantics apply
- audit-sensitive actions should use explicit confirmation and traceable UX copy

### 23.9 Initial Development Baseline

Before feature work expands, the frontend foundation should provide:

- unauthenticated and authenticated route groups
- a shared shell layout with header, sidebar, and content container
- theme bootstrap from `packages/design-tokens`
- reusable page header, empty state, and resource table primitives
- `src/start.ts` with global request middleware for auth, tracing, and request context
- a Storybook surface for shell and form components
- a typed API integration pattern that the first CRUD flows can reuse

---

## 35. Frontend Module Breakdown

### 35.1 `packages/design-tokens`

Owns Mantine theme creation, semantic tokens, and visual system defaults.

Recommended exports:

- `createAppTheme()`
- semantic color map
- spacing and radius scales
- typography presets
- data-density tokens for console tables and forms

### 35.2 `packages/ui-kit`

Owns shared presentational and composite components built on Mantine.

Recommended structure:

- `primitives/` for thin opinionated wrappers
- `composites/` for reusable app patterns such as resource tables and settings sections
- `shell/` for navigation, page chrome, breadcrumbs, and headers
- `feedback/` for empty states, inline alerts, confirmations, and async status surfaces

### 35.3 `packages/ts-api-client`

Generated client for the control plane API with typed responses.

### 35.4 `packages/ts-shared-schema`

Shared frontend-safe schemas for forms, filters, and API validation.

### 35.5 `apps/console-web`

The main product UI.

Key modules:

- auth
- navigation shell
- projects and credentials
- provider resources
- route editor
- usage explorer
- billing dashboard
- audit explorer
- diagnostics center
- system health

Recommended app-level foldering:

- `src/routes` for route files
- `src/features` for product-domain features
- `src/lib` for app wiring and client configuration
- `src/styles` for app-specific CSS modules and global resets that Mantine does not cover

---
