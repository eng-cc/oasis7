// Async renderers can focus their canvas after an account dialog has mounted.
// Keep the active modal's focus boundary valid as its controls arrive.
export function keepFocusInDialog(getDialog) {
  function guard(event) {
    const dialog = getDialog();
    if (!dialog?.isConnected || dialog.contains(event.target)) return;
    const first = dialog.querySelector('input:not([disabled]),button:not([disabled]),[tabindex="0"]');
    (first || dialog).focus();
  }
  document.addEventListener("focusin", guard);
  return () => document.removeEventListener("focusin", guard);
}
