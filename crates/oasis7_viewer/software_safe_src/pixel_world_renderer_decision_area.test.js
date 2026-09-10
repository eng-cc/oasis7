import { describe, expect, it } from 'vitest';
import { rendererDecisionMaxHeight } from './pixel_world_renderer_decision_area.js';

describe('renderer decision panel clearance', () => {
  const rect = (left,top,right,bottom) => ({left,top,right,bottom});
  it('puts desktop and narrow event glyphs above the scrollable panel', () => {
    expect(rendererDecisionMaxHeight(rect(16,0,776,884),900,[rect(367,590,411,634)])).toBe(242);
    expect(rendererDecisionMaxHeight(rect(10,0,380,834),844,[rect(90,551,134,595)])).toBe(231);
  });
  it('keeps a usable panel for near-bottom events and ignores offscreen events', () => {
    const panel = rect(10,0,380,834);
    expect(rendererDecisionMaxHeight(panel,844,[rect(90,798,134,842)])).toBe(160);
    expect(rendererDecisionMaxHeight(panel,844,[rect(90,850,134,894)])).toBe(346);
    expect(rendererDecisionMaxHeight(panel,844,[rect(400,551,444,595)])).toBe(346);
  });
});
