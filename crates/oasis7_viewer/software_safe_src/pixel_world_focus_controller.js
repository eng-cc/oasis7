export function createPixelWorldFocusController({
  focusMode,
  commandDrawerOpen,
  diagnosticsDrawerOpen,
  setFocusMode,
  setCommandDrawerOpen,
  setDiagnosticsDrawerOpen,
  setMaximized,
}) {
  let focusInvoker = null;
  let commandDrawerInvoker = null;
  let diagnosticsDrawerInvoker = null;

  function rememberInvoker(event) {
    const candidate = event?.currentTarget instanceof HTMLElement
      ? event.currentTarget
      : document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    return candidate;
  }

  function restoreFocus(invoker) {
    if (!invoker) {
      return;
    }
    queueMicrotask(() => {
      if (document.contains(invoker)) {
        invoker.focus();
      }
    });
  }

  function enterFocusMode(event) {
    focusInvoker = rememberInvoker(event);
    setFocusMode(true);
    setCommandDrawerOpen(false);
    setDiagnosticsDrawerOpen(false);
    setMaximized(false);
  }

  function exitFocusMode() {
    const invoker = focusInvoker;
    setFocusMode(false);
    setCommandDrawerOpen(false);
    setDiagnosticsDrawerOpen(false);
    setMaximized(false);
    restoreFocus(invoker);
  }

  function closeCommandDrawer({ returnFocus = false } = {}) {
    setCommandDrawerOpen(false);
    if (returnFocus) {
      restoreFocus(commandDrawerInvoker);
    }
  }

  function closeDiagnosticsDrawer({ returnFocus = false } = {}) {
    setDiagnosticsDrawerOpen(false);
    if (returnFocus) {
      restoreFocus(diagnosticsDrawerInvoker);
    }
  }

  function openCommandDrawer(event) {
    commandDrawerInvoker = rememberInvoker(event);
    setCommandDrawerOpen(true);
    setDiagnosticsDrawerOpen(false);
  }

  function openDiagnosticsDrawer(event) {
    diagnosticsDrawerInvoker = rememberInvoker(event);
    setDiagnosticsDrawerOpen(true);
    setCommandDrawerOpen(false);
  }

  function handleKeyDown(event) {
    if (event.key !== "Escape" || !focusMode() || event.isComposing) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (commandDrawerOpen()) {
      closeCommandDrawer({ returnFocus: true });
      return;
    }
    if (diagnosticsDrawerOpen()) {
      closeDiagnosticsDrawer({ returnFocus: true });
      return;
    }
    exitFocusMode();
  }

  return {
    enterFocusMode,
    exitFocusMode,
    openCommandDrawer,
    openDiagnosticsDrawer,
    closeCommandDrawer,
    closeDiagnosticsDrawer,
    handleKeyDown,
  };
}
