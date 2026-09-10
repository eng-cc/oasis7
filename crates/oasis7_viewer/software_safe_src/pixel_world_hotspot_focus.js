const FOCUSABLE = 'button, a[href], input, select, textarea, summary, [tabindex]';

function firstSummary(details) {
  return [...details.children].find((child) => child.tagName === 'SUMMARY');
}

function isAvailableControl(node) {
  if (node.tabIndex < 0 || node.disabled) return false;
  if (node.tagName === 'SUMMARY' && !node.hasAttribute('tabindex')
      && (node.parentElement?.tagName !== 'DETAILS' || firstSummary(node.parentElement) !== node)) return false;
  for (let ancestor = node; ancestor; ancestor = ancestor.parentElement) {
    const style = getComputedStyle(ancestor);
    if (ancestor.hidden || ancestor.inert || style.display === 'none' || style.visibility === 'hidden') return false;
    if (ancestor.tagName === 'DETAILS' && !ancestor.open && !firstSummary(ancestor)?.contains(node)) return false;
  }
  return true;
}

export function moveFocusFromHotspotTooltip(tooltip, backwards) {
  const trigger = [...document.querySelectorAll('.pixel-world-hotspot')]
    .find((node) => node.getAttribute('aria-describedby') === tooltip.id);
  if (!trigger || !isAvailableControl(trigger)) return false;
  if (backwards) {
    trigger.focus();
    return true;
  }
  const controls = [...document.querySelectorAll(FOCUSABLE)]
    .filter((node) => !tooltip.contains(node) && isAvailableControl(node));
  const next = controls[controls.indexOf(trigger) + 1];
  if (!next || next === trigger) return false;
  next.focus();
  return document.activeElement === next;
}
