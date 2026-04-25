import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { resetSessionForTests, signIn } from "../features/auth/session";
import { setConsoleDataServiceForTests } from "../features/control-plane/service";
import { useControlPlaneFetchMock } from "./control-plane-fetch";
import { renderRoute } from "./router-test-utils";

describe("merchant center route", () => {
  useControlPlaneFetchMock();

  beforeEach(() => {
    resetSessionForTests();
    setConsoleDataServiceForTests(null);
  });

  it("renders replay evidence and opens replay capsule detail", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/merchant");

    expect(
      await screen.findByRole("heading", {
        name: "Merchant Center",
      }),
    ).toBeInTheDocument();
    expect(screen.getByText("Replay Evidence")).toBeInTheDocument();
    expect(screen.getByText("Tokens saved")).toBeInTheDocument();
    expect(screen.getByText("2,400")).toBeInTheDocument();
    expect(screen.getByText("Fingerprint pass")).toBeInTheDocument();
    expect(screen.getByText("Protocol warning")).toBeInTheDocument();
    expect(screen.getByText("Token warning")).toBeInTheDocument();
    expect(screen.getByText("Multimodal not_tested")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "View replay" }));

    expect(
      await screen.findByText("req_merchant_eval_acme"),
    ).toBeInTheDocument();
    expect(screen.getByText("Selected")).toBeInTheDocument();
    expect(screen.getByText("trace_merchant_eval_acme")).toBeInTheDocument();
    expect(screen.getByText("provider_signature_mismatch")).toBeInTheDocument();
  });

  it("refreshes replay evidence after running a relay evaluation", async () => {
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });

    await renderRoute("/app/merchant");

    fireEvent.click(
      await screen.findByRole("button", { name: "Run evaluation" }),
    );

    expect(
      await screen.findByText(
        "Recorded evaluation reval_2 with replay capsule replay_2.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByText("4,800")).toBeInTheDocument();
    expect(screen.getByText("2 of 2 evaluations shown.")).toBeInTheDocument();
    expect(screen.getByText("replay_2")).toBeInTheDocument();
  });
});
