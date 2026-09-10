import { expect, it } from 'vitest';
import { pixelWorldRoutesAndEventsVisualFixture, pixelWorldSelectedBlockerVisualFixture } from './pixel_world_visual_fixture_data.js';

it('publishes explicit current assignment authority while retaining an honest no-route control', () => {
  const route = pixelWorldRoutesAndEventsVisualFixture();
  for (const agent of Object.values(route.model.agents)) {
    expect(agent.relation).toEqual({kind:'agent_assignment',status:'active',source_class:'runtime_projection',freshness:'current'});
    expect(route.model.locations[agent.location_id].pos).not.toEqual(agent.pos);
  }
  expect(pixelWorldSelectedBlockerVisualFixture().model.agents['agent-0'].relation).toBeUndefined();
});
