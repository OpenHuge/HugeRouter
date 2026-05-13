import { afterEach, beforeEach, vi } from "vitest";
import { handleControlPlaneRequest } from "./control-plane-fetch-request-handler";
import { createInitialControlPlaneMockState } from "./control-plane-fetch-state";

export function createControlPlaneFetchMock() {
  const state = createInitialControlPlaneMockState();

  return vi.fn<
    (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>
  >((input, init) =>
    Promise.resolve(handleControlPlaneRequest(state, input, init)),
  );
}

export function useControlPlaneFetchMock() {
  let fetchMock: ReturnType<typeof createControlPlaneFetchMock>;

  beforeEach(() => {
    fetchMock = createControlPlaneFetchMock();
    vi.stubGlobal("fetch", fetchMock);
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  return () => fetchMock;
}
