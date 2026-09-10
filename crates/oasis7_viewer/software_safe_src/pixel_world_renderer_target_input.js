export function forwardRendererTargetPointer(event) {
  const canvas = event.currentTarget.closest('.pixel-world-canvas')?.querySelector('canvas');
  if (!canvas) return;
  // Capture belongs to the renderer, so subsequent drag/up events keep the
  // same native pointer identity and use its existing click-suppression path.
  canvas.dispatchEvent(new PointerEvent('pointerdown', {
    clientX:event.clientX, clientY:event.clientY, pointerId:event.pointerId,
    pointerType:event.pointerType, button:event.button, buttons:event.buttons,
    isPrimary:event.isPrimary,
  }));
}
