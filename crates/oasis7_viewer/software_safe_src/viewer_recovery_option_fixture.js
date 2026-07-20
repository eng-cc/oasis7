export function recoveryOptionVisualFixture() {
  return [
    {
      kind: "repair",
      estimated_time_class: "short",
      estimated_resource_class: "focused_local_input",
      risk_class: "low",
      retained_benefit: "Retains the current local line and operating context.",
      recommendation_reason: "Use repair when the blocker is localized.",
    },
    {
      kind: "rebuild",
      estimated_time_class: "medium",
      estimated_resource_class: "broader_local_reinvestment",
      risk_class: "moderate",
      retained_benefit: "Retains local ownership while replacing the fragile arrangement.",
      recommendation_reason: "Use rebuild when the line cannot absorb the blocker.",
    },
    {
      kind: "pivot",
      estimated_time_class: "medium",
      estimated_resource_class: "redirected_local_commitment",
      risk_class: "tradeoff",
      retained_benefit: "Retains independent progress through a new specialization.",
      recommendation_reason: "Use pivot when a different local path avoids the pressure.",
    },
  ];
}
