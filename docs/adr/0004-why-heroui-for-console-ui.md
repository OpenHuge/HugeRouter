# ADR 0004: Why HeroUI for Console UI

[Back to ADR Index](README.md)

## Status

Accepted

## Context

The control plane is an admin-heavy product with dense forms, tables, filters, dialogs, confirmations, and diagnostics surfaces. We need a component foundation that:

- lets the team ship real management workflows quickly
- provides strong default accessibility and interaction patterns
- supports cohesive theming without forcing a large custom design system up front
- works well with TanStack Start, TanStack Query, and typed forms
- still allows product-specific composition in shared packages such as `packages/ui-kit`

The previous draft architecture left the frontend styling layer open enough that multiple implementation styles could emerge, which would slow early development and fragment shared UI work.

## Decision

Use `HeroUI` as the default frontend component foundation for the console.

We will standardize on:

- HeroUI for layout, inputs, overlays, notifications, and general admin UI primitives
- `packages/design-tokens` for project theme creation and semantic tokens
- `packages/ui-kit` for shared HeroUI-based wrappers and composite console components
- TanStack Form + Zod as the primary typed form stack, rendered through HeroUI components

## Alternatives Considered

### Tailwind CSS plus custom component assembly

This approach offers flexibility, but it pushes too much design-system and interaction work onto the team at the start of the project. For an operations console, the cost of assembling reliable primitives is higher than the benefit of starting fully custom.

### Headless UI plus custom styling

Headless primitives help with accessibility, but they still require a substantial styling and composition layer. That is better suited to a product with a mature bespoke design language than to our current phase.

### Other component libraries

Other libraries could work, but HeroUI provides a strong balance of admin-console ergonomics, theming, layout primitives, and practical developer experience for the current team and scope.

## Consequences

Positive:

- faster delivery of authenticated shell, forms, dialogs, and feedback states
- consistent interaction patterns across tenant and admin surfaces
- reduced need to build low-level primitives from scratch
- easier Storybook coverage and shared component reuse

Tradeoffs:

- design language stays somewhat constrained by HeroUI defaults unless we invest in stronger theming
- wrapper discipline is required so `packages/ui-kit` does not become a leaky duplicate of upstream HeroUI
- upstream library upgrades need compatibility checks for shared wrappers and theme tokens

## Follow-up Actions

- define the initial theme in `packages/design-tokens`
- build shell, feedback, and form primitives in `packages/ui-kit`
- wire the root app provider stack around `UiProvider` and HeroUI feedback surfaces
- add component-library verification to CI and Storybook workflows
