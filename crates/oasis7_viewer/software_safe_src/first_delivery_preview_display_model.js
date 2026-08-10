function displayableStrings(value) {
  return Array.isArray(value)
    ? value
      .filter((entry) => typeof entry === "string")
      .map((entry) => entry.trim())
      .filter(Boolean)
    : [];
}

function displayableString(value) {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function humanizeIdentifier(value) {
  const humanized = value.split("_").filter(Boolean).join(" ");
  return humanized ? `${humanized[0].toUpperCase()}${humanized.slice(1)}` : humanized;
}

function humanizeRequiredInput(value) {
  const separator = " × ";
  const quantityAt = value.indexOf(separator);
  return quantityAt === -1
    ? humanizeIdentifier(value)
    : `${humanizeIdentifier(value.slice(0, quantityAt))}${value.slice(quantityAt)}`;
}

export function normalizeFirstDeliveryPreview(value) {
  if (value == null || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  return {
    localNeed: displayableString(value.local_need || value.localNeed),
    expectedOutput: displayableString(value.expected_output || value.expectedOutput),
    requiredInputs: displayableStrings(value.required_inputs || value.requiredInputs).map(humanizeRequiredInput),
    valueTiming: displayableString(value.value_timing || value.valueTiming),
    leverageClassUnlocked: (() => {
      const valueToDisplay = displayableString(value.leverage_class_unlocked || value.leverageClassUnlocked);
      return valueToDisplay ? humanizeIdentifier(valueToDisplay) : null;
    })(),
    returnVisitHook: displayableString(value.return_visit_hook || value.returnVisitHook),
  };
}
