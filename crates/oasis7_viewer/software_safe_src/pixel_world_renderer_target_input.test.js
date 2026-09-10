import { afterEach, expect, it, vi } from 'vitest';
import { forwardRendererTargetPointer } from './pixel_world_renderer_target_input.js';

afterEach(()=>vi.unstubAllGlobals());
it('delegates the original pointer identity and coordinates to renderer capture', () => {
  class Pointer extends Event { constructor(type,properties){super(type);Object.assign(this,properties);} }
  vi.stubGlobal('PointerEvent',Pointer);
  const root=document.createElement('div');root.className='pixel-world-canvas';
  root.innerHTML='<canvas></canvas><button></button>';
  const received=vi.fn();root.querySelector('canvas').addEventListener('pointerdown',received);
  forwardRendererTargetPointer({currentTarget:root.querySelector('button'),clientX:120,clientY:210,pointerId:27,pointerType:'touch',button:0,buttons:1,isPrimary:true});
  expect(received.mock.calls[0][0]).toMatchObject({clientX:120,clientY:210,pointerId:27,pointerType:'touch',buttons:1});
});
