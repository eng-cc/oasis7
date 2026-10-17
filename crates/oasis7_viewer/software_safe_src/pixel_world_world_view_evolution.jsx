import { For, Show } from 'solid-js';
export function PixelWorldEvolutionExplorer(props) {
  const trusted = event => !props.stale && event.source_class === 'runtime_journal_payload'
    && ['current','last_known'].includes(event.freshness) && event.source_cut === props.view?.source_cut
    && event.identity?.world_id === props.view?.world_id && String(event.identity?.reorg_epoch) === String(props.view?.reorg_epoch)
    && event.position && ['x_cm','y_cm','z_cm'].every(key=>typeof event.position[key] === 'string' && /^-?(0|[1-9]\d*)$/.test(event.position[key]));
  const selectedEvent=()=>props.view?.events?.find(event=>props.selection?.kind === 'world_event' && event.event_id === props.selection.id);
  const selectedAggregate=()=>props.view?.aggregates?.find(value=>props.selection?.kind === 'industry_aggregate' && value.aggregate_id === props.selection.id);
  return <>
    <Show when={props.view?.aggregates?.length}><section aria-label={props.text('工业区域','Industrial areas')}>
      <h4>{props.text('工业区域','Industrial areas')}</h4>
      <For each={props.view.aggregates}>{aggregate=><div>
        <button type="button" onClick={()=>props.onSelect?.({kind:'industry_aggregate',id:aggregate.aggregate_id})}>{aggregate.label}</button>
        <span> · {aggregate.member_count} {props.text('个对象','objects')} · {props.freshnessLabel(aggregate.provenance?.freshness)}</span>
        <button type="button" onClick={()=>props.onDrilldown?.(aggregate)}>{props.text('展开区域','Explore area')}</button>
      </div>}</For>
    </section></Show>
    <Show when={props.view?.events?.length}><section aria-label={props.text('空间事件','Spatial events')}>
      <h4>{props.text('空间事件','Spatial events')}</h4>
      <p>{props.text('近期获准事件锚点，并非完整事件历史。','Recent authorized anchors, not a complete event history.')}</p>
      <For each={props.view.events}>{event=><div>
        <button type="button" onClick={()=>props.onSelect?.({kind:'world_event',id:event.event_id})}>{event.label}</button>
        <span> · {props.freshnessLabel(event.freshness)}</span>
        <button type="button" disabled={!trusted(event)} onClick={()=>props.onLocate?.({kind:'world_event',id:event.event_id})}>{props.text('定位事件','Locate event')}</button>
        <button type="button" onClick={()=>props.onEventFeed?.(event)}>{props.text('查看世界动态','View world feed')}</button>
      </div>}</For>
    </section></Show>
    <Show when={selectedAggregate()}>{value=><section class="pixel-world-spatial-inspection" tabindex="-1" aria-label={props.text('区域检查','Area inspection')}>
      <strong>{value().label}</strong><p>{props.text('只读区域汇总','Read-only area summary')} · {value().member_count} {props.text('个对象','objects')}</p>
    </section>}</Show>
    <Show when={selectedEvent()}>{value=><section class="pixel-world-spatial-inspection" tabindex="-1" aria-label={props.text('事件检查','Event inspection')}>
      <strong>{value().label}</strong><p>{props.text('历史事件锚点，仅供观察','Historical event anchor; read-only')} · {props.freshnessLabel(value().freshness)}</p>
      <p>{props.text('逻辑时间','Logical time')}: {value().logical_time}</p>
    </section>}</Show>
  </>;
}
