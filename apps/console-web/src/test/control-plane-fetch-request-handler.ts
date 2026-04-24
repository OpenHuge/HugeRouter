import type { ControlPlaneMockState } from "./control-plane-fetch-state";
import { jsonResponse, resolvePath } from "./control-plane-fetch-route-utils";
import { handleMerchantRequest } from "./control-plane-fetch-merchant-handlers";
import {
  handleMutationRequest,
  handleReadOnlyRequest,
} from "./control-plane-fetch-resource-handlers";

export function handleControlPlaneRequest(
  state: ControlPlaneMockState,
  input: RequestInfo | URL,
  init?: RequestInit,
) {
  const path = resolvePath(input);

  const merchantResponse = handleMerchantRequest(state, path, init);
  if (merchantResponse) {
    return merchantResponse;
  }

  if (
    !init?.method ||
    init.method === "GET" ||
    (path === "/v1/route-simulations" && init.method === "POST")
  ) {
    const readOnlyResponse = handleReadOnlyRequest(state, path);
    if (readOnlyResponse) {
      return readOnlyResponse;
    }
  }

  const mutationResponse = handleMutationRequest(state, path, init);
  if (mutationResponse) {
    return mutationResponse;
  }

  return jsonResponse(404, {
    error: {
      code: "not_found",
      message: `Unhandled test path ${path}`,
      request_id: "req_test",
      retryable: false,
    },
  });
}
