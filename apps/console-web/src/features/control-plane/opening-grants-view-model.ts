import type { ConfigSnapshotView } from "./types";

export type OpeningGrantFormState = {
  configSnapshotId: string;
  expiresAt: string;
  granteeId: string;
  granteeKind: "user" | "workspace" | "agent";
  granteeLabel: string;
  ownerAccountId: string;
  scopes: string;
};

export type OpeningGrantFormErrors = Partial<
  Record<keyof OpeningGrantFormState, string>
>;

export type ExistingOpeningGrant = {
  expiresAt: string;
  granteeId: string;
  granteeKind: string;
  isActive: boolean;
  ownerAccountId: string;
  projectId: string;
};

export type OpeningGrantCreateInput = {
  configSnapshotId: string;
  expiresAt: string;
  granteeId: string;
  granteeKind: "user" | "workspace" | "agent";
  granteeLabel?: string;
  ownerAccountId: string;
  scopes: string[];
};

export type OpeningGrantValidationContext = {
  existingGrants?: ExistingOpeningGrant[];
  now?: Date;
  snapshots?: ConfigSnapshotView[];
};

export type OpeningGrantOwnerSummary = {
  activeCount: number;
  ownerAccountId: string;
  projectIds: string[];
  totalCount: number;
};

export function defaultOpeningGrantExpiresAt(now = new Date()) {
  return new Date(now.getTime() + 30 * 24 * 60 * 60 * 1000)
    .toISOString()
    .replace(/\.\d{3}Z$/, "Z");
}

export function createOpeningGrantFormState(
  snapshots: ConfigSnapshotView[],
): OpeningGrantFormState {
  const preferredSnapshot =
    snapshots.find((snapshot) => snapshot.status === "active") ?? snapshots[0];

  return {
    configSnapshotId: preferredSnapshot?.configSnapshotId ?? "",
    expiresAt: defaultOpeningGrantExpiresAt(),
    granteeId: "",
    granteeKind: "user",
    granteeLabel: "",
    ownerAccountId: "",
    scopes: "route:codex,provider:hugerouter-commercial",
  };
}

export function parseOpeningGrantScopes(value: string) {
  return value
    .split(",")
    .map((scope) => scope.trim())
    .filter(Boolean);
}

function selectedProjectId(
  form: OpeningGrantFormState,
  snapshots: ConfigSnapshotView[] = [],
) {
  return snapshots.find(
    (snapshot) => snapshot.configSnapshotId === form.configSnapshotId,
  )?.projectId;
}

function isActiveOpeningGrant(
  grant: Pick<ExistingOpeningGrant, "expiresAt" | "isActive">,
  now: Date,
) {
  if (!grant.isActive) {
    return false;
  }

  const expiresAt = Date.parse(grant.expiresAt);
  return Number.isNaN(expiresAt) || expiresAt > now.getTime();
}

function matchesOwnerScope(
  grant: ExistingOpeningGrant,
  ownerAccountId: string,
  projectId?: string,
) {
  return (
    grant.ownerAccountId === ownerAccountId &&
    (projectId === undefined || grant.projectId === projectId)
  );
}

export function countActiveOpeningGrants(
  grants: ExistingOpeningGrant[],
  scope: { ownerAccountId?: string; projectId?: string } = {},
  now = new Date(),
) {
  return grants.filter((grant) => {
    if (!isActiveOpeningGrant(grant, now)) {
      return false;
    }

    return scope.ownerAccountId
      ? matchesOwnerScope(grant, scope.ownerAccountId, scope.projectId)
      : true;
  }).length;
}

export function summarizeOpeningGrantOwners(
  grants: ExistingOpeningGrant[],
  now = new Date(),
): OpeningGrantOwnerSummary[] {
  const summaries = new Map<string, OpeningGrantOwnerSummary>();

  for (const grant of grants) {
    const current = summaries.get(grant.ownerAccountId) ?? {
      activeCount: 0,
      ownerAccountId: grant.ownerAccountId,
      projectIds: [],
      totalCount: 0,
    };

    current.totalCount += 1;
    if (!current.projectIds.includes(grant.projectId)) {
      current.projectIds.push(grant.projectId);
    }

    if (isActiveOpeningGrant(grant, now)) {
      current.activeCount += 1;
    }

    summaries.set(grant.ownerAccountId, current);
  }

  return [...summaries.values()].sort((left, right) =>
    left.ownerAccountId.localeCompare(right.ownerAccountId),
  );
}

export function buildOpeningGrantCreateInput(
  form: OpeningGrantFormState,
): OpeningGrantCreateInput {
  const granteeLabel = form.granteeLabel.trim();

  return {
    configSnapshotId: form.configSnapshotId,
    expiresAt: form.expiresAt.trim(),
    granteeId: form.granteeId.trim(),
    granteeKind: form.granteeKind,
    granteeLabel: granteeLabel === "" ? undefined : granteeLabel,
    ownerAccountId: form.ownerAccountId.trim(),
    scopes: parseOpeningGrantScopes(form.scopes),
  };
}

export function validateOpeningGrantForm(
  form: OpeningGrantFormState,
  context: OpeningGrantValidationContext = {},
) {
  const errors: OpeningGrantFormErrors = {};
  const ownerPattern = /^[A-Za-z0-9_.-]{1,128}$/;
  const normalized = buildOpeningGrantCreateInput(form);
  const projectId = selectedProjectId(form, context.snapshots);
  const existingGrants = context.existingGrants ?? [];
  const now = context.now ?? new Date();

  if (!form.configSnapshotId) {
    errors.configSnapshotId = "Select a sale-ready config snapshot.";
  }

  if (!ownerPattern.test(normalized.ownerAccountId)) {
    errors.ownerAccountId =
      "Use 1-128 letters, digits, underscore, hyphen, or dot.";
  }

  if (!normalized.granteeId) {
    errors.granteeId = "Enter the grantee id.";
  }

  if (!normalized.expiresAt) {
    errors.expiresAt = "Enter an expiration timestamp.";
  } else if (Number.isNaN(Date.parse(normalized.expiresAt))) {
    errors.expiresAt = "Enter a valid RFC3339 timestamp.";
  }

  if (normalized.scopes.length === 0) {
    errors.scopes = "Enter at least one scope.";
  }

  if (!errors.ownerAccountId && projectId) {
    const activeOwnerCount = countActiveOpeningGrants(
      existingGrants,
      {
        ownerAccountId: normalized.ownerAccountId,
        projectId,
      },
      now,
    );

    if (activeOwnerCount >= 8) {
      errors.ownerAccountId =
        "This owner account already has 8 active child keys.";
    }
  }

  if (!errors.granteeId && !errors.ownerAccountId && projectId) {
    const duplicate = existingGrants.some(
      (grant) =>
        matchesOwnerScope(grant, normalized.ownerAccountId, projectId) &&
        grant.granteeKind === normalized.granteeKind &&
        grant.granteeId === normalized.granteeId &&
        isActiveOpeningGrant(grant, now),
    );

    if (duplicate) {
      errors.granteeId =
        "This grantee already has an active child key for this owner account.";
    }
  }

  return errors;
}
