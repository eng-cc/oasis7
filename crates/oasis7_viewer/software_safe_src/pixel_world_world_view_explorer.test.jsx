import { render, screen, fireEvent, cleanup } from '@solidjs/testing-library';
import { afterEach, describe, it, expect, vi } from 'vitest';
import { createSignal } from 'solid-js';
import { PixelWorldWorldViewExplorer } from './pixel_world_world_view_explorer.jsx';
afterEach(cleanup);
const entity=(id,freshness='current')=>({entity_id:id,kind:'agent',label:id,position:{x_cm:'10',y_cm:'20',z_cm:'0'},provenance:{geometry_source:'authoritative',freshness,valid_at_source_cut:'cut'}});
describe('world exploration',()=>{
  it('offers separate overlapping candidates and does not enable unknown geometry',()=>{
    const onSelect=vi.fn();render(()=> <PixelWorldWorldViewExplorer locale="en" state={{view:{source_cut:'cut',entities:[entity('A'),entity('B'),entity('Unknown','unknown')],completeness:'partial'}}} onSelect={onSelect}/>);
    fireEvent.click(screen.getByRole('button',{name:'B',exact:true}));expect(onSelect).toHaveBeenCalledWith({kind:'agent',id:'B'});
    expect(screen.getAllByRole('button',{name:'Follow'})[2].disabled).toBe(true);expect(screen.getByRole('status').textContent).toContain('partial');
  });
  it('pauses follow when source expires without automatically resuming',async()=>{
    const onFollow=vi.fn();const [state,setState]=createSignal({view:{source_cut:'cut',entities:[entity('A')]}});
    render(()=> <PixelWorldWorldViewExplorer locale="en" state={state()} onFollow={onFollow}/>);
    fireEvent.click(screen.getByRole('button',{name:'Follow'}));expect(onFollow).toHaveBeenLastCalledWith({kind:'agent',id:'A'});
    setState({view:{source_cut:'cut',entities:[entity('A','expired')]}});await Promise.resolve();expect(onFollow).toHaveBeenLastCalledWith(null);
    setState({view:{source_cut:'cut',entities:[entity('A')]}});await Promise.resolve();expect(screen.getByRole('button',{name:'Follow'}).getAttribute('aria-pressed')).toBe('false');
  });
});

it('searches the server scope and exposes explicit return actions',()=>{
 const onQuery=vi.fn(),onOverview=vi.fn(),onReturnTarget=vi.fn();
 render(()=> <PixelWorldWorldViewExplorer locale="en" state={{view:{source_cut:'cut',entities:[],completeness:'unknown'}}} onQuery={onQuery} onOverview={onOverview} onReturnTarget={onReturnTarget}/>);
 fireEvent.input(screen.getByRole('textbox'),{target:{value:' mine '}});fireEvent.submit(screen.getByRole('textbox').closest('form'));
 expect(onQuery).toHaveBeenLastCalledWith({search:'mine'});
 fireEvent.click(screen.getByRole('button',{name:'World overview'}));expect(onOverview).toHaveBeenCalled();expect(onQuery).toHaveBeenLastCalledWith({});
 fireEvent.click(screen.getByRole('button',{name:'Return to target'}));expect(onReturnTarget).toHaveBeenCalled();
});
it('queries a bounded region using the fixed world basis',()=>{
 const onQuery=vi.fn();render(()=> <PixelWorldWorldViewExplorer locale="en" state={{view:{source_cut:'cut',world_bounds:{width_cm:'800',depth_cm:'400'},entities:[entity('A')]}}} onQuery={onQuery}/>);
 fireEvent.click(screen.getByRole('button',{name:'Explore region'}));
 expect(onQuery).toHaveBeenCalledWith({bounds:{min_x_cm:'0',max_x_cm:'110',min_y_cm:'0',max_y_cm:'70'}});
});
it('clears the follow indicator on manual camera stop',async()=>{
 const [stop,setStop]=createSignal(0);render(()=> <PixelWorldWorldViewExplorer locale="en" pauseVersion={stop()} state={{view:{source_cut:'cut',entities:[entity('A')]}}}/>);
 fireEvent.click(screen.getByRole('button',{name:'Follow'}));expect(screen.getByRole('button',{name:'Follow'}).getAttribute('aria-pressed')).toBe('true');
 setStop(1);await Promise.resolve();expect(screen.getByRole('button',{name:'Follow'}).getAttribute('aria-pressed')).toBe('false');
});

it('stops following before returning to the main target',()=>{
 const order=[];render(()=> <PixelWorldWorldViewExplorer locale="en" state={{view:{source_cut:'cut',entities:[entity('B')]}}} onFollow={value=>order.push(value?'follow':'stop')} onReturnTarget={()=>order.push('return')}/>);
 fireEvent.click(screen.getByRole('button',{name:'Follow'}));
 fireEvent.click(screen.getByRole('button',{name:'Return to target'}));
 expect(order).toEqual(['follow','stop','return']);
 expect(screen.getByRole('button',{name:'Follow'}).getAttribute('aria-pressed')).toBe('false');
});
it('inspects an authorized facility with derived position without offering Agent follow or controls',()=>{
 const facility={...entity('F'),kind:'factory',label:'Furnace',provenance:{...entity('F').provenance,geometry_source:'location_derived'}};
 const onSelect=vi.fn(),onLocate=vi.fn();render(()=> <PixelWorldWorldViewExplorer locale="en" state={{view:{source_cut:'cut',entities:[facility]}}} onSelect={onSelect} onLocate={onLocate}/>);
 fireEvent.click(screen.getByRole('button',{name:'Furnace'}));expect(onSelect).toHaveBeenCalledWith({kind:'factory',id:'F'});
 expect(screen.getByRole('region',{name:'Object inspection'})).toHaveTextContent('Read-only observation');
 expect(screen.getByRole('button',{name:'Follow'}).disabled).toBe(true);fireEvent.click(screen.getByRole('button',{name:'Locate'}));
 expect(onLocate).toHaveBeenCalledWith({kind:'factory',id:'F'},{x_cm:10,y_cm:20,z_cm:0});expect(screen.queryByRole('button',{name:/Submit|Build|Chat/})).not.toBeInTheDocument();
});
it('offers read-only route endpoint inspection only when both endpoints are in the same authorized view',()=>{
 const from=entity('A'),to={...entity('F'),kind:'factory',label:'Furnace'};
 const route={relation_id:'r',kind:'logistics_route',label:'Ore delivery',source_class:'runtime_projection',status:'unavailable',freshness:'unknown',valid_at_source_cut:'cut',from:{kind:'agent',id:'A'},to:{kind:'factory',id:'F'}};
 const onSelect=vi.fn();const [state,setState]=createSignal({view:{source_cut:'cut',entities:[from,to],relations:[route]}});
 render(()=> <PixelWorldWorldViewExplorer locale="en" state={state()} onSelect={onSelect}/>);
 fireEvent.click(screen.getByRole('button',{name:'Ore delivery'}));expect(screen.getByRole('region',{name:'Route inspection'})).toHaveTextContent('read-only');
 fireEvent.click(screen.getByRole('button',{name:'Inspect Furnace'}));expect(onSelect).toHaveBeenCalledWith({kind:'factory',id:'F'});
 setState({view:{source_cut:'cut',entities:[from],relations:[route]}});expect(screen.queryByRole('region',{name:'Authorized routes'})).not.toBeInTheDocument();
});
