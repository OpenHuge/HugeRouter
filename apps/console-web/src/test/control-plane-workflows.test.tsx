import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { resetSessionForTests, signIn } from "../features/auth/session";
import { setConsoleDataServiceForTests } from "../features/control-plane/service";
import { renderRoute } from "./router-test-utils";
import { useControlPlaneFetchMock } from "./control-plane-fetch";

describe("control-plane operational workflows", () => {
  useControlPlaneFetchMock();

  beforeEach(() => {
    resetSessionForTests();
    setConsoleDataServiceForTests(null);
    signIn({
      email: "tenant@acme.dev",
      workspace: "acme-retail",
    });
  });

  it("creates, edits, validates, and disables provider resources", async () => {
    await renderRoute("/app/providers");

    fireEvent.click(screen.getByRole("button", { name: "Create provider" }));
    fireEvent.click(screen.getByRole("button", { name: "Create provider" }));

    expect(
      await screen.findByText(/starts with prvrsrc_/i),
    ).toBeInTheDocument();
    expect(screen.getByText("Enter a provider resource name.")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Provider resource id"), {
      target: { value: "prvrsrc_openai_canary" },
    });
    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "OpenAI Canary" },
    });
    fireEvent.change(screen.getByLabelText("Provider id"), {
      target: { value: "openai" },
    });
    fireEvent.change(screen.getByLabelText("Region"), {
      target: { value: "eu-west-1" },
    });
    fireEvent.change(screen.getByLabelText("Endpoint URL"), {
      target: { value: "https://api.openai.com/v1" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create provider" }));

    expect(
      await screen.findByText("Created provider resource OpenAI Canary."),
    ).toBeInTheDocument();
    expect(await screen.findByText("OpenAI Canary")).toBeInTheDocument();

    const canaryRow = screen.getByRole("row", { name: /OpenAI Canary/ });
    fireEvent.click(within(canaryRow).getByRole("button", { name: "Edit" }));
    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "OpenAI Canary Updated" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save provider" }));

    expect(
      await screen.findByText("Updated provider resource OpenAI Canary Updated."),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("OpenAI Canary Updated"),
    ).toBeInTheDocument();

    const updatedRow = screen.getByRole("row", {
      name: /OpenAI Canary Updated/,
    });
    fireEvent.click(within(updatedRow).getByRole("button", { name: "Disable" }));

    await waitFor(() => {
      const disabledRow = screen.getByRole("row", {
        name: /OpenAI Canary Updated/,
      });
      expect(within(disabledRow).getAllByText("disabled").length).toBeGreaterThan(0);
    });
  });

  it("shows route policy compatibility guidance, then creates, edits, and disables a route policy", async () => {
    await renderRoute("/app/routes");

    fireEvent.click(
      screen.getByRole("button", { name: "Create route policy" }),
    );

    expect(
      screen.getByText(/Supported capability values:/),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Route policy id"), {
      target: { value: "routepol_openai_canary" },
    });
    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "OpenAI Canary Route" },
    });
    fireEvent.change(screen.getByLabelText("Model alias"), {
      target: { value: "reasoning-canary" },
    });
    fireEvent.change(screen.getByLabelText("Required capabilities"), {
      target: { value: "json_mode, unsupported_cap" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Create route policy" }),
    );

    expect(
      await screen.findByText(/Unsupported capability values:/),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Required capabilities"), {
      target: { value: "json_mode, tool_calling" },
    });
    fireEvent.change(screen.getByLabelText("Preferred regions"), {
      target: { value: "us-east-1, eu-west-1" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: "Create route policy" }),
    );

    expect(
      await screen.findByText("Created route policy OpenAI Canary Route."),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("OpenAI Canary Route"),
    ).toBeInTheDocument();

    const createdRow = screen.getByRole("row", {
      name: /OpenAI Canary Route/,
    });
    fireEvent.click(within(createdRow).getByRole("button", { name: "Edit" }));
    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "OpenAI Canary Route Updated" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save route policy" }));

    expect(
      await screen.findByText("Updated route policy OpenAI Canary Route Updated."),
    ).toBeInTheDocument();
    expect(
      await screen.findByText("OpenAI Canary Route Updated"),
    ).toBeInTheDocument();

    const updatedRow = screen
      .getByText("OpenAI Canary Route Updated")
      .closest("tr");
    expect(updatedRow).not.toBeNull();
    fireEvent.click(
      within(updatedRow as HTMLTableRowElement).getByRole("button", {
        name: "Disable",
      }),
    );

    await waitFor(() => {
      expect(
        screen.queryByText("OpenAI Canary Route Updated"),
      ).not.toBeInTheDocument();
    });
  });

  it("validates, creates, and activates config snapshots with a refreshed list", async () => {
    await renderRoute("/app/snapshots");

    fireEvent.click(screen.getByRole("button", { name: "Create snapshot" }));
    fireEvent.click(screen.getByRole("button", { name: "Save snapshot" }));

    expect(
      await screen.findByText(/starts with cfgsnap_/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Select at least one provider resource."),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Snapshot id"), {
      target: { value: "cfgsnap_gateway_v3" },
    });
    fireEvent.change(screen.getByLabelText("Revision"), {
      target: { value: "5" },
    });
    fireEvent.click(
      screen.getByRole("checkbox", {
        name: /OpenAI Primary \(prvrsrc_openai_primary\)/,
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Save snapshot" }));

    expect(
      await screen.findByText("Created config snapshot cfgsnap_gateway_v3."),
    ).toBeInTheDocument();
    expect(await screen.findByText("cfgsnap_gateway_v3")).toBeInTheDocument();

    const snapshotRow = screen.getByRole("row", { name: /cfgsnap_gateway_v3/ });
    fireEvent.click(
      within(snapshotRow).getByRole("button", { name: "Activate" }),
    );

    expect(
      await screen.findByText("Activated snapshot cfgsnap_gateway_v3."),
    ).toBeInTheDocument();
    await waitFor(() => {
      const activeRow = screen.getByText("cfgsnap_gateway_v3").closest("tr");
      expect(activeRow).not.toBeNull();
      expect(
        within(activeRow as HTMLTableRowElement).getByText("active"),
      ).toBeInTheDocument();
    });
  });

  it("validates, creates, previews once, and revokes API keys", async () => {
    await renderRoute("/app/api-keys");

    fireEvent.click(screen.getByRole("button", { name: "Create API key" }));
    fireEvent.click(screen.getByRole("button", { name: "Save API key" }));

    expect(
      await screen.findByText("Enter an API key label."),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Enter the full secret value before saving."),
    ).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Display name"), {
      target: { value: "Canary API Key" },
    });
    fireEvent.change(screen.getByLabelText("Secret value"), {
      target: { value: "akp_canary_secret_value" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save API key" }));

    expect(await screen.findByText("Save this secret now")).toBeInTheDocument();
    expect(
      screen.getByText("Secret: akp_canary_secret_value"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Prefix: akp_ca/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Return to list" }));

    expect(await screen.findByText("Canary API Key")).toBeInTheDocument();

    const keyRow = screen.getByRole("row", { name: /Canary API Key/ });
    fireEvent.click(within(keyRow).getByRole("button", { name: "Revoke" }));

    await waitFor(() => {
      const revokedRow = screen.getByRole("row", { name: /Canary API Key/ });
      expect(within(revokedRow).getByText("Revoked")).toBeInTheDocument();
    });
  });

  it("queues billing exports, shows pending status, refreshes to completed, and downloads the export", async () => {
    const createObjectUrl = vi.fn(() => "blob:billing-export");
    const revokeObjectUrl = vi.fn();
    const clickSpy = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(() => {});
    const originalCreateObjectUrl = URL.createObjectURL;
    const originalRevokeObjectUrl = URL.revokeObjectURL;
    URL.createObjectURL = createObjectUrl;
    URL.revokeObjectURL = revokeObjectUrl;

    await renderRoute("/app/billing");

    fireEvent.click(screen.getByRole("button", { name: "Queue export" }));

    expect(await screen.findByText("Queued a billing export job.")).toBeInTheDocument();
    expect(await screen.findByText("export_201")).toBeInTheDocument();

    const queuedRow = screen.getByRole("row", { name: /export_201/ });
    expect(within(queuedRow).getByText("Pending")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Refresh exports" }));

    await waitFor(() => {
      const completedRow = screen.getByRole("row", { name: /export_201/ });
      expect(
        within(completedRow).getByRole("button", { name: "Download" }),
      ).toBeInTheDocument();
    });

    const completedRow = screen.getByRole("row", { name: /export_201/ });
    fireEvent.click(
      within(completedRow).getByRole("button", { name: "Download" }),
    );

    expect(
      await screen.findByText("Downloaded export export_201.csv."),
    ).toBeInTheDocument();
    expect(createObjectUrl).toHaveBeenCalled();
    expect(revokeObjectUrl).toHaveBeenCalled();

    URL.createObjectURL = originalCreateObjectUrl;
    URL.revokeObjectURL = originalRevokeObjectUrl;
    clickSpy.mockRestore();
  });
});
