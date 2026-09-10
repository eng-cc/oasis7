import { afterEach, describe, expect, it } from 'vitest';
import { moveFocusFromHotspotTooltip } from './pixel_world_hotspot_focus.js';

afterEach(() => { document.body.innerHTML = ''; });
describe('portaled hotspot focus order', () => {
  it('continues from the final hotspot to the first native summary in closed details', () => {
    document.body.innerHTML = '<button class="pixel-world-hotspot" aria-describedby="tip">Info</button><details><summary id="feed">Feed</summary><button>Hidden reload</button></details><div id="tip"><button>Close</button></div>';
    expect(moveFocusFromHotspotTooltip(document.getElementById('tip'), false)).toBe(true);
    expect(document.activeElement.id).toBe('feed');
  });
  it('skips nested closed bodies and secondary summaries while honoring a closed first summary subtree', () => {
    document.body.innerHTML = '<details><summary><button class="pixel-world-hotspot" aria-describedby="tip">Origin</button><button id="allowed">Summary action</button></summary><summary tabindex="0">Second summary</summary><details open><summary>Nested hidden</summary><button>Hidden</button></details></details><button id="outside">Outside</button><div id="tip"><button>Close</button></div>';
    const tip = document.getElementById('tip');
    expect(moveFocusFromHotspotTooltip(tip, false)).toBe(true);
    expect(document.activeElement.id).toBe('allowed');
    document.getElementById('allowed').disabled = true;
    expect(moveFocusFromHotspotTooltip(tip, false)).toBe(true);
    expect(document.activeElement.id).toBe('outside');
  });
  it('returns backward to its trigger and continues forward past hidden controls', () => {
    document.body.innerHTML = '<button class="pixel-world-hotspot" aria-describedby="explanation">Goal</button><div hidden><button>Hidden</button></div><button disabled>Disabled</button><button id="next">Next action</button><div id="explanation"><button>Close</button></div>';
    const tooltip = document.getElementById('explanation');
    expect(moveFocusFromHotspotTooltip(tooltip, true)).toBe(true);
    expect(document.activeElement.textContent).toBe('Goal');
    expect(moveFocusFromHotspotTooltip(tooltip, false)).toBe(true);
    expect(document.activeElement.id).toBe('next');
  });
  it('includes a nested disclosure summary when its outer details is open', () => {
    document.body.innerHTML = '<button class="pixel-world-hotspot" aria-describedby="tip">Info</button><details open><summary tabindex="-1">Outer</summary><details><summary id="nested">Nested</summary><button>Hidden</button></details></details><div id="tip"><button>Close</button></div>';
    expect(moveFocusFromHotspotTooltip(document.getElementById('tip'), false)).toBe(true);
    expect(document.activeElement.id).toBe('nested');
  });
});
