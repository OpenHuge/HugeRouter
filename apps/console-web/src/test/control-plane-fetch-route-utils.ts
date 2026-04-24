export const emptyResponse = (status: number) =>
  new Response("", {
    status,
    headers: {
      "content-type": "application/json",
    },
  });

export function jsonResponse(status: number, payload: unknown) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: {
      "content-type": "application/json",
    },
  });
}

export function resolvePath(input: RequestInfo | URL) {
  if (typeof input === "string") {
    return new URL(input, "http://127.0.0.1").pathname;
  }

  if (input instanceof URL) {
    return input.pathname;
  }

  return new URL(input.url, "http://127.0.0.1").pathname;
}

export function parseRequestBody(init?: RequestInit) {
  if (!init?.body || typeof init.body !== "string") {
    return null;
  }

  return JSON.parse(init.body) as Record<string, unknown>;
}
