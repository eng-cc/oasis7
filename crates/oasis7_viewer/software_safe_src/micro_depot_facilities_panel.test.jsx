import { cleanup, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";

import { MicroDepotFacilitiesPanel } from "./micro_depot_facilities_panel.jsx";

const tr = (_locale, zh, en) => en;

describe("MicroDepotFacilitiesPanel", () => {
  it("shows canonical state, module, receipt, proposal, and runtime-published actions", () => {
    render(() => (
      <MicroDepotFacilitiesPanel
        locale={() => "en"}
        tr={tr}
        facilities={[{
          facilityId: "depot-1", ownerClaimId: "claim-1", status: "active", locationId: "loc-0",
          serviceRadiusCm: 1200, inventoryRevision: 3, availableUnitsByKind: { data: 6 },
          throughputEpoch: 4, throughputRemainingUnits: 12, throughputLimitUnitsPerEpoch: 16,
          supportedResourceKinds: ["data"], moduleId: "micro_depot.eval.v2", moduleVersion: "v2",
          wasmHash: "0123456789abcdef0123456789abcdef", upkeepPaid: true,
          lastReceiptId: "receipt-1", lastProposalHash: "proposal-1",
          availableActions: ["service_micro_depot_repair"],
        }]}
      />
    ));

    const depotPanel = screen.getByTestId("micro-depot-facilities-panel");
    expect(within(depotPanel).getByTestId("micro-depot-facility-depot-1")).toHaveTextContent("data=6");
    expect(within(depotPanel).getByText("micro_depot.eval.v2")).toBeInTheDocument();
    expect(within(depotPanel).getByText("receipt-1")).toBeInTheDocument();
    expect(within(depotPanel).getByText("proposal=proposal-1")).toBeInTheDocument();
    expect(within(depotPanel).getByText("service_micro_depot_repair")).toBeInTheDocument();
  });

  it("hides absent or empty facility lists", () => {
    render(() => (
      <MicroDepotFacilitiesPanel locale={() => "en"} tr={tr} facilities={null} />
    ));
    expect(screen.queryByTestId("micro-depot-facilities-panel")).not.toBeInTheDocument();

    cleanup();
    render(() => (
      <MicroDepotFacilitiesPanel locale={() => "en"} tr={tr} facilities={[]} />
    ));
    expect(screen.queryByTestId("micro-depot-facilities-panel")).not.toBeInTheDocument();
  });

  it("ignores malformed facility entries and safely degrades wrong-typed display fields", () => {
    render(() => (
      <MicroDepotFacilitiesPanel
        locale={() => "en"}
        tr={tr}
        facilities={[
          null,
          "not-a-facility",
          {
            facilityId: "depot-safe",
            availableUnitsByKind: "not-an-inventory-record",
            supportedResourceKinds: "data",
            availableActions: { action: "unsafe" },
          },
        ]}
      />
    ));

    const depotPanel = screen.getByTestId("micro-depot-facilities-panel");
    expect(within(depotPanel).getByTestId("micro-depot-facility-depot-safe")).toHaveTextContent("Inventory-");
    expect(within(depotPanel).getByText("The current snapshot publishes no available depot actions.")).toBeInTheDocument();
    expect(within(depotPanel).queryByText("data")).not.toBeInTheDocument();
  });
});
