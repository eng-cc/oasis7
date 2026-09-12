import { expect, it } from 'vitest';
import { pixelWorldRoutesAndEventsVisualFixture, pixelWorldSelectedBlockerVisualFixture } from './pixel_world_visual_fixture_data.js';

it('publishes explicit current assignment authority while retaining an honest no-route control', () => {
  const route = pixelWorldRoutesAndEventsVisualFixture();
  for (const id of ['agent-0', 'agent-1']) {
    const agent = route.model.agents[id];
    expect(agent.relation).toEqual({kind:'agent_assignment',status:'active',source_class:'runtime_projection',freshness:'current'});
    expect(route.model.locations[agent.location_id].pos).not.toEqual(agent.pos);
  }
  expect(route.model.agents['agent-route'].relation).toEqual({kind:'logistics_route',label:'Ore logistics route',status:'active',source_class:'runtime_projection',freshness:'current'});
  expect(route.model.agents['agent-unknown-route'].relation.kind).toBe('unknown');
  expect(route.model.agents['agent-stale-route'].relation.freshness).toBe('stale');
  expect(route.model.agents['agent-zero-route'].pos).toEqual(route.model.locations['loc-zero-route'].pos);
  expect(pixelWorldSelectedBlockerVisualFixture().model.agents['agent-0'].relation).toBeUndefined();
});
