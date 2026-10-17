import { createEffect } from 'solid-js';
import { PixelWorldWorldViewExplorer } from './pixel_world_world_view_explorer.jsx';

export function PixelWorldSpatialExplorer(props) {
  let root, openedSelection = null;
  createEffect(() => {
    const selected=props.controller.selection();
    const key=selected?.kind === 'factory' ? selected.id : null;
    if (key === openedSelection) return;
    openedSelection=key;
    if (!key) return;
    queueMicrotask(() => {
      if (!root?.isConnected) return;
      const disclosure=root.closest('details');if (disclosure) disclosure.open=true;
      root.querySelector('.pixel-world-entity-inspection')?.focus();
    });
  });
  return <div ref={root} class="pixel-world-exploration-panel"><PixelWorldWorldViewExplorer state={props.controller.state()} locale={props.locale}
    selection={props.controller.selection()} pauseVersion={props.controller.pauseVersion()}
    onQuery={props.controller.query} onSelect={props.controller.select}
    onLocate={props.controller.locate} onFollow={props.controller.follow}
    onOverview={props.controller.overview} onReturnTarget={props.controller.returnTarget} /></div>;
}
