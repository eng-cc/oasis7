import { cleanup, fireEvent, render, screen, waitFor } from '@solidjs/testing-library';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import LocalTools from './LocalTools';
const api=vi.hoisted(()=>({info:vi.fn(),bindings:vi.fn(),list:vi.fn(),get:vi.fn(),save:vi.fn(),signer:vi.fn(),jobs:vi.fn(),cli:vi.fn(),installCli:vi.fn(),chooseFile:vi.fn(),prepare:vi.fn(),reconcilePrepare:vi.fn(),submit:vi.fn(),inspect:vi.fn(),snapshot:vi.fn()}));
vi.mock('./bridge',async importOriginal=>({...await importOriginal<typeof import('./bridge')>(),connected:()=>true,observedTime:(v:string)=>v,contextBridge:api,toolsBridge:api}));
const binding={id:'bound',bindingVersion:1,root:'/private/context',storeId:'store',projectId:'oasis7',registeredProjectId:'project',name:'private',directoryIdentity:'identity'};
const metadata={schema_version:'oasis7.context_entry.v1',project_id:'oasis7',id:'machine',kind:'machine',title:'<img src=x onerror=alert(1)>',environment:'local',owner:'human',tags:['preserved'],state:'active',updated_at:'2026-10-12T00:00:00Z',updated_by:'human',verification:null,credential_refs:[{id:'vault',provider:'native',locator:'opaque',usage:'secret reference'}],links:[]};
const raw=`---\n${JSON.stringify(metadata)}\n---\n<script>evil()</script>`;
beforeEach(()=>{api.bindings.mockResolvedValue([binding]);api.signer.mockResolvedValue({state:'not_installed'});api.jobs.mockResolvedValue([]);api.list.mockResolvedValue({entries:[{id:'machine',title:metadata.title,kind:'machine',environment:'local',verification_state:'unverified',summary:'<script>evil()</script>'}],warnings:[]});api.get.mockResolvedValue({id:'machine',revision:'sha256:original',rawText:raw,metadataJson:JSON.stringify(metadata),verificationState:'unverified'});api.save.mockRejectedValue(new Error('REVISION_CONFLICT'));});
afterEach(()=>{cleanup();vi.clearAllMocks();});
async function openEntry(){render(()=> <LocalTools/>);await waitFor(()=>expect(screen.getByRole('option',{name:/private ·/})).toBeInTheDocument());await waitFor(()=>expect(screen.getByRole('button',{name:'检查本机事实'})).not.toBeDisabled());fireEvent.change(screen.getByLabelText('资料库'),{target:{value:'bound'}});await waitFor(()=>expect(screen.getByRole('button',{name:metadata.title})).toBeInTheDocument());await waitFor(()=>expect(screen.getByRole('button',{name:'搜索资料'})).not.toBeDisabled());fireEvent.click(screen.getByRole('button',{name:metadata.title}));await waitFor(()=>expect(screen.getByLabelText('资料正文')).toHaveValue('<script>evil()</script>'));}
describe('private library rendering and conflict handling',()=>{
  it('renders malicious titles and bodies as inert text',async()=>{await openEntry();expect(document.querySelector('img')).toBeNull();expect(document.querySelector('script')).toBeNull();expect(screen.getByLabelText('资料名称')).toHaveValue(metadata.title);});
  it('keeps draft, revision and undisplayed legal metadata after conflict',async()=>{await openEntry();fireEvent.input(screen.getByLabelText('资料正文'),{target:{value:'my unsaved edit'}});fireEvent.click(screen.getByRole('button',{name:'保存资料'}));await waitFor(()=>expect(screen.getByRole('status')).toHaveTextContent('REVISION_CONFLICT'));expect(screen.getByLabelText('资料正文')).toHaveValue('my unsaved edit');const request=api.save.mock.calls[0][0];expect(request.expectedRevision).toBe('sha256:original');expect(request.rawText).toContain('opaque');expect(request.rawText).toContain('preserved');expect(request.rawText).toContain('my unsaved edit');api.get.mockResolvedValueOnce({id:'machine',revision:'sha256:new',rawText:raw.replace('evil()','new CLI edit'),metadataJson:JSON.stringify(metadata),verificationState:'unverified'});fireEvent.click(screen.getByRole('button',{name:'比较当前修订与草稿'}));await waitFor(()=>expect(screen.getByLabelText('资料冲突比较')).toHaveTextContent('sha256:new'));expect(screen.getByLabelText('资料正文')).toHaveValue('my unsaved edit');expect(screen.getByLabelText('资料冲突比较')).toHaveTextContent('opaque');fireEvent.click(screen.getByRole('button',{name:'关闭比较并保留草稿'}));expect(screen.getByLabelText('资料正文')).toHaveValue('my unsaved edit');});
  it.each(['save', 'readback'])('retains typing during delayed %s and retries against the exact saved revision', async phase => {
    await openEntry();
    let finishSave!: (value: { revision: string; persistence: string; changed: boolean }) => void;
    let finishRead!: (value: object) => void;
    api.save.mockImplementationOnce(() => new Promise(resolve => { finishSave = resolve; }));
    api.get.mockImplementationOnce(() => new Promise(resolve => { finishRead = resolve; }));
    fireEvent.input(screen.getByLabelText('资料正文'), { target: { value: 'submitted draft' } });
    fireEvent.click(screen.getByRole('button', { name: '保存资料' }));
    await waitFor(() => expect(api.save).toHaveBeenCalledTimes(1));
    if (phase === 'save') fireEvent.input(screen.getByLabelText('资料正文'), { target: { value: 'later draft' } });
    finishSave({ revision: 'sha256:saved', persistence: 'saved', changed: true });
    await waitFor(() => expect(api.get).toHaveBeenCalledTimes(2));
    if (phase === 'readback') fireEvent.input(screen.getByLabelText('资料正文'), { target: { value: 'later draft' } });
    // CLI writes after our save: readback is not an approved CAS baseline for the later draft.
    finishRead({ id: 'machine', revision: 'sha256:external', rawText: raw, metadataJson: JSON.stringify(metadata), verificationState: 'unverified' });
    await waitFor(() => expect(screen.getByRole('button', { name: '保存资料' })).toBeEnabled());
    expect(screen.getByLabelText('资料正文')).toHaveValue('later draft');
    expect(screen.getByText(/保存期间的新草稿已保留/)).toBeInTheDocument();
    expect(api.save).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button', { name: '保存资料' }));
    await waitFor(() => expect(api.save).toHaveBeenCalledTimes(2));
    expect(api.save.mock.calls[1][0].expectedRevision).toBe('sha256:saved');
    expect(api.save.mock.calls[1][0].rawText).toContain('later draft');
    expect(api.save.mock.calls[1][1]).not.toBe(api.save.mock.calls[0][1]);
    await screen.findByText('REVISION_CONFLICT');
    expect(screen.getByLabelText('资料正文')).toHaveValue('later draft');
  });
  it('keeps edits made while an explicit reload is awaiting the file', async () => {
    await openEntry();
    let finishRead!: (value: object) => void;
    api.get.mockImplementationOnce(() => new Promise(resolve => { finishRead = resolve; }));
    fireEvent.click(screen.getByRole('button', { name: '重新加载并放弃草稿' }));
    await waitFor(() => expect(api.get).toHaveBeenCalledTimes(2));
    fireEvent.input(screen.getByLabelText('资料名称'), { target: { value: 'new title while loading' } });
    finishRead({ id: 'machine', revision: 'sha256:external', rawText: raw, metadataJson: JSON.stringify(metadata), verificationState: 'unverified' });
    await waitFor(() => expect(screen.getByRole('button', { name: '保存资料' })).toBeEnabled());
    expect(screen.getByLabelText('资料名称')).toHaveValue('new title while loading');
    expect(screen.getByRole('status')).toHaveTextContent('读取期间的新草稿已保留');
    fireEvent.click(screen.getByRole('button', { name: '保存资料' }));
    await waitFor(() => expect(api.save).toHaveBeenCalled());
    expect(api.save.mock.calls[0][0].expectedRevision).toBe('sha256:original');
  });
  it('retains selected library, draft and original binding version across CLI and overview refresh',async()=>{
    await openEntry();
    const refreshButton=screen.getByRole('button',{name:'检查本机事实'});
    await waitFor(()=>expect(refreshButton).not.toBeDisabled());
    fireEvent.input(screen.getByLabelText('资料正文'),{target:{value:'draft pinned to original binding'}});
    api.installCli.mockResolvedValue({path:'/tools/context'});
    api.bindings.mockResolvedValue([{...binding,bindingVersion:2}]);
    fireEvent.click(screen.getByRole('button',{name:'从本 App 启用资料 CLI'}));
    await waitFor(()=>expect(refreshButton).not.toBeDisabled());
    expect(screen.getByLabelText('资料库')).toHaveValue('bound');
    expect(screen.getByLabelText('资料正文')).toHaveValue('draft pinned to original binding');
    fireEvent.click(refreshButton);
    await waitFor(()=>expect(api.bindings).toHaveBeenCalledTimes(3));
    await waitFor(()=>expect(refreshButton).not.toBeDisabled());
    expect(screen.getByLabelText('资料库')).toHaveValue('bound');
    expect(screen.getByLabelText('资料正文')).toHaveValue('draft pinned to original binding');
    expect(api.get).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button',{name:'保存资料'}));
    await waitFor(()=>expect(api.save).toHaveBeenCalled());
    expect(api.save.mock.calls[0][0].bindingVersion).toBe(1);
  });
  it('localizes normal signer diagnostics while retaining the actual diagnostic in details',async()=>{
    api.signer.mockResolvedValue({state:'needs_configuration',reason:'App is not the bound ordinary caller'});
    render(()=> <LocalTools/>);
    await waitFor(()=>expect(screen.getByText('当前 App 用户尚未绑定为签名调用者。')).toBeInTheDocument());
    expect(screen.getByText(/App is not the bound ordinary caller/).closest('details')).not.toBeNull();
  });
});
async function fillSigning(context='{}'){
  api.signer.mockResolvedValue({state:'available'});api.chooseFile.mockResolvedValue('/selected/source');
  render(()=> <LocalTools/>);
  await waitFor(()=>expect(screen.getByRole('button',{name:'选择待签名文件'})).not.toBeDisabled());
  fireEvent.click(screen.getByRole('button',{name:'选择待签名文件'}));
  await waitFor(()=>expect(screen.getByRole('button',{name:'检查本机事实'})).not.toBeDisabled());
  fireEvent.input(screen.getByLabelText('签名 Job ID'),{target:{value:'original-job'}});
  fireEvent.input(screen.getByLabelText('签名身份'),{target:{value:'signer-one'}});
  fireEvent.input(screen.getByLabelText('实际 grant ID'),{target:{value:'grant-one'}});
  fireEvent.input(screen.getByLabelText('完整签名上下文'),{target:{value:context}});
}
describe('prepare effect boundary',()=>{
  it('allows correction and a new operation after proven no-effect preflight failure',async()=>{
    api.prepare.mockRejectedValueOnce({message:'invalid context',effect:'none'});
    await fillSigning('null');
    fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(screen.getByText(/本次准备未创建或修改签名 Job/)).toBeInTheDocument());
    expect(screen.getByLabelText('完整签名上下文')).not.toBeDisabled();
    expect(screen.getByRole('button',{name:'准备固定请求'})).not.toBeDisabled();
    expect(screen.getByLabelText('签名 Job ID')).toHaveValue('original-job');
    const originalOperation=api.prepare.mock.calls[0][1];
    fireEvent.input(screen.getByLabelText('完整签名上下文'),{target:{value:'{"deployment_id":"corrected"}'}});
    api.prepare.mockRejectedValueOnce({message:'still invalid',effect:'none'});
    fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(api.prepare).toHaveBeenCalledTimes(2));
    expect(api.prepare.mock.calls[1][1]).not.toBe(originalOperation);
    expect(api.prepare.mock.calls[1][0].job_id).toBe('original-job');
  });
  it.each([{message:'write failed after job creation',effect:'unknown'},new Error('transport lost')])('keeps original job frozen when no-effect cannot be established: %j',async error=>{
    api.prepare.mockRejectedValueOnce(error);await fillSigning();
    fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(screen.getByText(/无法确认本次准备是否产生副作用/)).toBeInTheDocument());
    expect(screen.getByLabelText('签名 Job ID')).toBeDisabled();
    expect(screen.getByLabelText('签名 Job ID')).toHaveValue('original-job');
    expect(screen.getByLabelText('完整签名上下文')).toBeDisabled();
    expect(screen.getByRole('button',{name:'准备固定请求'})).toBeDisabled();
    expect(screen.getByRole('button',{name:'核对原准备操作'})).not.toBeDisabled();
    expect(api.prepare).toHaveBeenCalledTimes(1);
  });
  it('recovers the exact prepared preview after a lost response without unlocking or replaying',async()=>{
    const recovered={job_id:'original-job',prepared_fingerprint:'original-frozen-identity',file_sha256:'file-digest',payload_sha256:'payload-digest',operation_key:'original-key',work_dir:'/original/job',result:{status:'PREPARED'}};
    api.prepare.mockRejectedValueOnce(new Error('transport lost'));api.reconcilePrepare.mockResolvedValueOnce(recovered);api.submit.mockResolvedValue({status:'submitted'});
    await fillSigning();fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(screen.getByText(/无法确认本次准备是否产生副作用/)).toBeInTheDocument());
    const originalOperation=api.prepare.mock.calls[0][1];
    fireEvent.click(screen.getByRole('button',{name:'核对原准备操作'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'提交此请求'})).not.toBeDisabled());
    expect(api.reconcilePrepare.mock.calls[0][1]).toBe(originalOperation);
    expect(api.reconcilePrepare.mock.calls[0][0]).toEqual(api.prepare.mock.calls[0][0]);
    expect(screen.getByLabelText('签名 Job ID')).toBeDisabled();expect(screen.getByLabelText('完整签名上下文')).toBeDisabled();
    expect(screen.getByRole('button',{name:'准备固定请求'})).toBeDisabled();expect(api.prepare).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button',{name:'提交此请求'}));
    await waitFor(()=>expect(api.submit).toHaveBeenCalled());
    expect(api.submit.mock.calls[0].slice(0,2)).toEqual(['original-job','original-frozen-identity']);
  });
  it('keeps malformed JSON editable without invoking native prepare',async()=>{
    await fillSigning('{');fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(screen.getByRole('status')).toBeInTheDocument());
    expect(screen.getByLabelText('完整签名上下文')).not.toBeDisabled();
    expect(api.prepare).not.toHaveBeenCalled();
  });
});

describe('restore without a source library',()=>{
  it('offers encrypted recovery and submits no binding on a fresh machine',async()=>{
    api.bindings.mockResolvedValue([]);api.snapshot.mockResolvedValue({storeId:'restored-store'});
    render(()=> <LocalTools project={{id:'project',path:'/repo',gitCommonDir:'/repo/.git',branch:'main',head:'head',environment:'local'}}/>);
    await waitFor(()=>expect(screen.getByRole('button',{name:'检查本机事实'})).not.toBeDisabled());
    fireEvent.input(screen.getByLabelText('快照口令'),{target:{value:'fresh-secret'}});
    fireEvent.input(screen.getByLabelText('恢复新目录名称'),{target:{value:'restored-library'}});
    const restore=screen.getByRole('button',{name:'从加密快照恢复新资料库'});expect(restore).not.toBeDisabled();fireEvent.click(restore);
    await waitFor(()=>expect(api.snapshot).toHaveBeenCalledWith('restore',null,'fresh-secret','restored-library',[]));
    expect(screen.getByLabelText('快照口令')).toHaveValue('');
  });
});

describe('new signing request after verified completion',()=>{
  const prepared={job_id:'original-job',prepared_fingerprint:'original-frozen-identity',file_sha256:'file-digest',payload_sha256:'payload-digest',operation_key:'original-key',work_dir:'/original/job',result:{status:'PREPARED'}};
  const committed={command:'inspect',status:'committed',error_code:null};
  it('requires an original verified receipt before clearing and uses fresh operation IDs for the next request',async()=>{
    api.prepare.mockResolvedValueOnce(prepared);api.submit.mockResolvedValue({status:'committed'});api.inspect.mockResolvedValueOnce(committed);
    await fillSigning();fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'提交此请求'})).not.toBeDisabled());
    const originalPrepare=api.prepare.mock.calls[0][1];
    fireEvent.click(screen.getByRole('button',{name:'提交此请求'}));
    await waitFor(()=>expect(api.submit).toHaveBeenCalledTimes(1));
    await waitFor(()=>expect(screen.getByRole('button',{name:'核对原 Job 结果'})).not.toBeDisabled());
    const originalSubmit=api.submit.mock.calls[0][2];
    expect(screen.getByRole('button',{name:'开始新的签名请求'})).toBeDisabled();
    fireEvent.click(screen.getByRole('button',{name:'核对原 Job 结果'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'开始新的签名请求'})).not.toBeDisabled());
    expect(api.inspect).toHaveBeenCalledWith('original-job','original-frozen-identity');
    expect(screen.getByLabelText('签名 Job ID')).toBeDisabled();
    fireEvent.click(screen.getByRole('button',{name:'开始新的签名请求'}));
    expect(screen.getByLabelText('签名 Job ID')).toHaveValue('');expect(screen.getByLabelText('签名 Job ID')).not.toBeDisabled();
    expect(screen.getByLabelText('签名身份')).toHaveValue('');expect(screen.getByLabelText('实际 grant ID')).toHaveValue('');expect(screen.getByLabelText('完整签名上下文')).toHaveValue('');
    expect(screen.getByRole('button',{name:'提交此请求'})).toBeDisabled();
    api.chooseFile.mockResolvedValueOnce('/next/source');fireEvent.click(screen.getByRole('button',{name:'选择待签名文件'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'检查本机事实'})).not.toBeDisabled());
    fireEvent.input(screen.getByLabelText('签名 Job ID'),{target:{value:'next-job'}});fireEvent.input(screen.getByLabelText('签名身份'),{target:{value:'next-signer'}});fireEvent.input(screen.getByLabelText('实际 grant ID'),{target:{value:'next-grant'}});fireEvent.input(screen.getByLabelText('完整签名上下文'),{target:{value:'{}'}});
    api.prepare.mockResolvedValueOnce({...prepared,job_id:'next-job',prepared_fingerprint:'next-identity'});
    fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(api.prepare).toHaveBeenCalledTimes(2));
    expect(api.prepare.mock.calls[1][1]).not.toBe(originalPrepare);
    await waitFor(()=>expect(screen.getByRole('button',{name:'提交此请求'})).not.toBeDisabled());
    fireEvent.click(screen.getByRole('button',{name:'提交此请求'}));await waitFor(()=>expect(api.submit).toHaveBeenCalledTimes(2));
    expect(api.submit.mock.calls[1][2]).not.toBe(originalSubmit);expect(api.submit.mock.calls[1].slice(0,2)).toEqual(['next-job','next-identity']);
  });
  it('keeps reserved, malformed, failed and lost inspection outcomes frozen',async()=>{
    api.prepare.mockResolvedValueOnce(prepared);await fillSigning();fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(screen.getByRole('button',{name:'提交此请求'})).not.toBeDisabled());
    for(const receipt of [{command:'inspect',status:'reserved',error_code:null},{status:'committed',error_code:null},{command:'inspect',status:'committed',error_code:'ERROR'}]){
      api.inspect.mockResolvedValueOnce(receipt);fireEvent.click(screen.getByRole('button',{name:'核对原 Job 结果'}));
      await waitFor(()=>expect(screen.getByRole('button',{name:'核对原 Job 结果'})).not.toBeDisabled());
      expect(screen.getByRole('button',{name:'开始新的签名请求'})).toBeDisabled();expect(screen.getByLabelText('签名 Job ID')).toBeDisabled();
    }
    api.inspect.mockRejectedValueOnce(new Error('lost response'));fireEvent.click(screen.getByRole('button',{name:'核对原 Job 结果'}));
    await waitFor(()=>expect(screen.getByRole('status')).toHaveTextContent('lost response'));
    expect(screen.getByRole('button',{name:'开始新的签名请求'})).toBeDisabled();expect(screen.getByLabelText('签名 Job ID')).toHaveValue('original-job');
  });
  it('cannot clear an unknown preparation with a generic committed inspection',async()=>{
    api.prepare.mockRejectedValueOnce(new Error('lost prepare'));api.inspect.mockResolvedValueOnce(committed);
    await fillSigning();fireEvent.click(screen.getByRole('button',{name:'准备固定请求'}));
    await waitFor(()=>expect(screen.getByText(/无法确认本次准备是否产生副作用/)).toBeInTheDocument());
    fireEvent.click(screen.getByRole('button',{name:'核对原 Job 结果'}));await waitFor(()=>expect(api.inspect).toHaveBeenCalled());
    expect(screen.getByRole('button',{name:'开始新的签名请求'})).toBeDisabled();expect(screen.getByLabelText('签名 Job ID')).toBeDisabled();
  });
});
