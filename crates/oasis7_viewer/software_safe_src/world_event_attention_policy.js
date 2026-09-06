const WORLD_SCOPED_CRISIS_RUNTIME_KINDS = new Set([
  "runtime.gameplay.crisis_spawned",
  "runtime.gameplay.crisis_resolved",
  "runtime.gameplay.crisis_timed_out",
]);

export function isWorldScopedCrisisRuntimeEvent(event) {
  const runtimeKind = event?.kind?.type === "RuntimeEvent"
    ? event?.kind?.data?.kind
    : null;
  return WORLD_SCOPED_CRISIS_RUNTIME_KINDS.has(runtimeKind);
}
