import { cleanup, fireEvent, render, screen, within } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";

import { MicroDepotFacilitiesPanel } from "./micro_depot_facilities_panel.jsx";

const tr = (_locale, zh, en) => en;

describe("MicroDepotFacilitiesPanel", () => {
  it("keeps status, inventory, and throughput primary while disclosing technical evidence on demand", () => {
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
    const facility = within(depotPanel).getByTestId("micro-depot-facility-depot-1");
    expect(facility).toHaveTextContent("active");
    expect(facility).toHaveTextContent("data=6");
    expect(facility).toHaveTextContent("12/16");

    const technicalEvidence = within(facility).getByTestId("micro-depot-technical-evidence");
    expect(technicalEvidence.tagName).toBe("DETAILS");
    expect(technicalEvidence).not.toHaveAttribute("open");
    const technicalSummary = within(technicalEvidence).getByText("Technical evidence").closest("summary");
    expect(technicalSummary).toBeTruthy();

    fireEvent.click(technicalSummary);
    expect(technicalEvidence).toHaveAttribute("open");
    expect(within(technicalEvidence).getByText("micro_depot.eval.v2")).toBeInTheDocument();
    expect(within(technicalEvidence).getByText("receipt-1")).toBeInTheDocument();
    expect(within(technicalEvidence).getByText("proposal=proposal-1")).toBeInTheDocument();

    const availability = within(facility).getByLabelText("Available depot actions");
    const actionBadge = within(availability).getByText("service_micro_depot_repair");
    expect(actionBadge).toHaveAttribute("data-action-availability", "published");
    expect(actionBadge.closest("button, a, input, select, textarea")).toBeNull();
  });

  it("explains unpaid upkeep and an empty inventory without presenting an action control", () => {
    render(() => (
      <MicroDepotFacilitiesPanel
        locale={() => "en"}
        tr={tr}
        facilities={[{
          facilityId: "depot-constrained", status: "degraded", availableUnitsByKind: {},
          throughputRemainingUnits: 0, throughputLimitUnitsPerEpoch: 16, upkeepPaid: false,
          availableActions: [],
        }]}
      />
    ));

    const facility = screen.getByTestId("micro-depot-facility-depot-constrained");
    expect(within(facility).getByText("Inventory is empty.")).toBeInTheDocument();
    expect(within(facility).getByText("Upkeep is unpaid; service availability may be constrained.")).toBeInTheDocument();
    expect(within(facility).getByText("The current snapshot publishes no available depot actions.")).toBeInTheDocument();
    expect(within(facility).queryByRole("button")).not.toBeInTheDocument();
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
