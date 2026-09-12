const noop = () => {};

export function createViewerRenderHookRegistry() {
  let primary = noop;
  const subscribers = new Set();
  return {
    set(nextHook) {
      primary = typeof nextHook === "function" ? nextHook : noop;
    },
    subscribe(nextHook) {
      if (typeof nextHook !== "function") return noop;
      subscribers.add(nextHook);
      return () => subscribers.delete(nextHook);
    },
    invoke() {
      primary();
      for (const subscriber of [...subscribers]) subscriber();
    },
  };
}
