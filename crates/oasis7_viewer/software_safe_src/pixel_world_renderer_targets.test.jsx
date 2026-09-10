import { render, fireEvent } from '@solidjs/testing-library';
import { describe, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import { PixelWorldRendererTargets, rendererEntityTargetStyle } from './pixel_world_renderer_targets.jsx';

describe('real renderer accessible projection', () => {
  it('keeps partially visible hit boxes at each edge until their full bounds leave the canvas', () => {
    for (const [x, y, visible] of [[-1,50,true],[101,50,true],[50,-1,true],[50,101,true],[-22,50,false],[122,50,false],[50,-22,false],[50,122,false]]) {
      const style = rendererEntityTargetStyle({id:'edge',pos:{x_cm:50,y_cm:50}}, {width_cm:100,depth_cm:100}, {width:100,height:100}, {pan_x_px:x-50,pan_y_px:y-50});
      expect(style.display, `${x},${y}`).toBe(visible ? undefined : 'none');
    }
  });
  const bounds = { width_cm: 1000, depth_cm: 1000 };
  const agent = { id: 'agent-0', pos: { x_cm: 250, y_cm: 600 } };
  it('retains focused targets across snapshots while updating data, geometry and membership', async () => {
    const other = { id: 'agent-1', pos: { x_cm: 100, y_cm: 200 } };
    const location = { id: agent.id, label: 'Depot', pos: { x_cm: 300, y_cm: 400 } };
    const snapshot = () => ({ world_bounds: bounds, agents: [{...agent}, {...other}], locations: [{...location}] });
    const [state, setState] = createSignal(snapshot());
    const [selection, setSelection] = createSignal(null);
    const select = vi.fn();
    const hover = vi.fn();
    const size = { width: 960, height: 540 };
    const view = render(() => <PixelWorldRendererTargets locale={() => 'en'} renderState={state} stageSize={() => size} selection={selection} onSelect={select} onHover={hover} />);
    const target = view.container.querySelector('[data-agent-id="agent-0"]');
    const depot = view.getByRole('button', {name: 'Select Depot'});
    target.focus();
    setState(snapshot());
    expect(view.container.querySelector('[data-agent-id="agent-0"]')).toBe(target);
    expect(document.activeElement).toBe(target);
    const moved = {...agent, label: 'Courier', pos: { x_cm: 700, y_cm: 300 }};
    setState({world_bounds: bounds, agents: [moved, {...other}], locations: [{...location, label: 'Warehouse'}]});
    expect(target).toHaveAccessibleName('Select Courier');
    expect(target.style.left).toBe(rendererEntityTargetStyle(moved, bounds, size).left);
    expect(document.activeElement).toBe(target);
    expect(view.getByRole('button', {name: 'Select Warehouse'})).toBe(depot);
    setSelection({kind: 'agent', id: agent.id});
    expect(target).toHaveAttribute('aria-pressed', 'true');
    expect(depot).toHaveAttribute('aria-pressed', 'false');
    setState({world_bounds: bounds, agents: [{...other}, {...moved}], locations: [{...location}]});
    expect([...view.container.querySelectorAll('[data-agent-id]')].map(node => node.dataset.agentId)).toEqual(['agent-1', 'agent-0']);
    expect(view.container.querySelector('[data-agent-id="agent-0"]')).toBe(target);
    await fireEvent.click(target);
    await fireEvent.mouseEnter(target);
    expect(select).toHaveBeenCalledWith({kind: 'agent', id: agent.id});
    expect(hover).toHaveBeenCalledWith({kind: 'agent', id: agent.id});
    setState({world_bounds: bounds, agents: [{...moved}], locations: []});
    expect(view.getByRole('button')).toBe(target);
    expect(depot.isConnected).toBe(false);
    setState({world_bounds: bounds, agents: [], locations: []});
    expect(target.isConnected).toBe(false);
    expect(view.queryByRole('button')).toBeNull();
  });
  it('normalizes Chinese locale aliases and does not synthesize absent renderer locations', () => {
    const view = render(() => <PixelWorldRendererTargets locale={() => 'zh-CN'} renderState={() => ({ agents: [{id:'agent-0'}], locations:[{id:'location-0',pos:{x_cm:1,y_cm:1}}] })} stageSize={() => ({width:960,height:540})} selection={() => null} onSelect={() => {}} onHover={() => {}} />);
    expect(view.getByRole('button')).toHaveAccessibleName('选择 行动体 0');
    expect(view.container.querySelector('[data-location-id]')).toBeNull();
  });
  it('centers the hit area over the camera-focused GPU agent rather than the old world-percent marker', () => {
    const style = rendererEntityTargetStyle(agent, bounds, { width: 1440, height: 900 }, { zoom: 1, pan_x_px: 350, pan_y_px: -86 });
    expect(style.left).toBe('50%');
    expect(style.top).toBe('50%');
    expect(style.transform).toBe('translate(-50%, -50%)');
    expect(style.width).toBe('44px');
  });
  it('updates pan, zoom and resize without changing selection identity', async () => {
    const [camera, setCamera] = createSignal({ zoom: 1, pan_x_px: 0, pan_y_px: 0 });
    const [size, setSize] = createSignal({ width: 1440, height: 900 });
    const select = vi.fn();
    const view = render(() => <PixelWorldRendererTargets locale={() => 'en'} renderState={() => ({ world_bounds: bounds, agents: [agent], locations: [] })} cameraState={camera} stageSize={size} selection={() => ({kind:'agent',id:'agent-0'})} onSelect={select} onHover={() => {}} />);
    const target = view.getByRole('button');
    const initial = target.style.left;
    setCamera({ zoom: 2, pan_x_px: 200, pan_y_px: -86 });
    expect(target.style.left).not.toBe(initial);
    setSize({ width: 390, height: 844 });
    expect(target.style.left).toBe(rendererEntityTargetStyle(agent,bounds,size(),camera()).left);
    await fireEvent.click(target);
    expect(select).toHaveBeenCalledWith({kind:'agent',id:'agent-0'});
    expect(view.container.querySelectorAll('[data-agent-id="agent-0"]')).toHaveLength(1);
  });
});
