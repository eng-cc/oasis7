import { createSignal, For, Show } from 'solid-js';
import { worldViewPosition, worldViewRelations } from './pixel_world_world_view.js';

export function PixelWorldEntityInspector(props) {
  const entity = () => props.view?.entities?.find(value => value.kind === props.selection?.kind && value.entity_id === props.selection?.id);
  const point = () => worldViewPosition(entity(),props.view,props.stale,true);
  return <Show when={entity()}><section aria-label={props.text('对象检查','Object inspection')} class="pixel-world-entity-inspection" tabindex="-1">
    <strong>{entity().label || entity().entity_id}</strong>
    <p>{props.text('只读观察','Read-only observation')} · {props.geometryLabel(entity().provenance?.geometry_source)} · {props.freshnessLabel(entity().provenance?.freshness)}</p>
    <Show when={point()} fallback={<p>{props.text('没有当前可信位置；不提供地图定位。','No current trusted position; map location unavailable.')}</p>}>
      <p>{props.text('显示坐标','Display coordinates')}: {point()?.x_cm}, {point()?.y_cm}, {point()?.z_cm} cm</p>
    </Show>
  </section></Show>;
}

export function PixelWorldRouteExplorer(props) {
  const [inspected,setInspected] = createSignal(null);
  const routes = () => worldViewRelations(props.view);
  const entity = ref => ref.kind === 'aggregate' ? props.view.aggregates?.find(value=>value.aggregate_id === ref.id) : props.view.entities.find(value => value.kind === ref.kind && value.entity_id === ref.id);
  const label = ref => entity(ref)?.label || ref.id;
  const route = () => routes().find(value => value.relation_id === inspected());
  const point = ref => ref.kind === 'aggregate' ? null : worldViewPosition(entity(ref),props.view,props.stale,true);
  const selection = ref => ({kind:ref.kind === 'aggregate' ? 'industry_aggregate' : ref.kind,id:ref.id});
  return <Show when={routes().length}><section aria-label={props.text('授权路线','Authorized routes')}>
    <h4>{props.text('路线与端点','Routes and endpoints')}</h4>
    <ul><For each={routes()}>{value=><li>
      <button type="button" onClick={()=>setInspected(value.relation_id)}>{value.label || value.relation_id}</button>
      <span>{value.status === 'active' ? props.text('可用','Available') : props.text('不可用','Unavailable')} · {props.freshnessLabel(value.freshness)}</span>
      <Show when={value.kind === 'industry_route'}><span> · {value.route_count} {props.text('条路线汇总','routes summarized')}</span></Show>
      <span>{label(value.from)} → {label(value.to)}</span>
    </li>}</For></ul>
    <Show when={route()}><div role="region" aria-label={props.text('路线检查','Route inspection')}>
      <p>{props.text('路线仅供观察，不会提交行动。','Routes are read-only and do not submit actions.')}</p>
      <For each={[route().from,route().to]}>{ref=><div>
        <button type="button" onClick={()=>props.onSelect?.(selection(ref))}>{props.text('检查','Inspect')} {label(ref)}</button>
        <button type="button" disabled={!point(ref)} onClick={()=>props.onLocate?.({kind:ref.kind,id:ref.id},point(ref))}>{props.text('定位','Locate')} {label(ref)}</button>
      </div>}</For>
    </div></Show>
  </section></Show>;
}
