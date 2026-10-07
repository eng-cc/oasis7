"""Fixed, invocation-scoped terminal observation bounds; no lifecycle effects."""
import base64
import contextlib
import contextvars
import datetime
import hashlib
import io
import json
import os
import selectors
import subprocess
import time
import zipfile

JSON_LIMIT=10*1024*1024
ARTIFACT_LIMIT=16*1024*1024
TOTAL_LIMIT=64*1024*1024
_scope=contextvars.ContextVar('terminal_recovery_observation',default=None)

class ObservationError(ValueError):
    def __init__(self,message,stdout=b'',stderr=b'',returncode=None):
        super().__init__(message)
        self.stdout=stdout
        self.stderr=stderr
        self.returncode=returncode

def canonical(value):
    return json.dumps(value,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()

def digest(raw):
    return hashlib.sha256(raw).hexdigest()

def _pairs(values):
    result={}
    for key,value in values:
        if key in result:raise ValueError('duplicate observation JSON key')
        result[key]=value
    return result

def load(raw):
    try:return json.loads(raw,object_pairs_hook=_pairs)
    except (UnicodeError,json.JSONDecodeError) as exc:raise ValueError('malformed observation JSON') from exc

def closed(value,fields,label):
    if type(value) is not dict or set(value)!=set(fields):
        raise ValueError(label+' closed schema mismatch')
    return value

def positive(value,label):
    if type(value) is not int or value<1:raise ValueError(label+' positive integer identity required')
    return value

def instant(value):
    if not isinstance(value,str):raise ValueError('observation timestamp unavailable')
    try:result=datetime.datetime.fromisoformat(value.replace('Z','+00:00'))
    except ValueError as exc:raise ValueError('invalid observation timestamp') from exc
    if result.tzinfo is None:raise ValueError('observation timestamp timezone required')
    return result

def now():
    # New observations use the server comment timestamp's second precision.
    # Historical/native timestamps are parsed unchanged.
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec='seconds')

class Budget:
    def __init__(self):
        self.deadline=time.monotonic()+300
        self.bytes=0
        self.evidence=[]
    def remaining(self):
        value=self.deadline-time.monotonic()
        if value<=0:raise ValueError('terminal observation aggregate deadline exceeded')
        return value
    def charge(self,raw):
        self.remaining();self.bytes+=len(raw)
        if self.bytes>TOTAL_LIMIT:raise ValueError('terminal observation aggregate byte limit exceeded')
    def retain(self,kind,locator,raw):
        self.evidence.append({'kind':kind,'locator':locator,
            'raw_b64':base64.b64encode(raw).decode(),'raw_sha256':digest(raw)})

def active():return _scope.get()

@contextlib.contextmanager
def observation():
    existing=active()
    if existing is not None:
        existing.remaining();yield existing;return
    token=_scope.set(Budget())
    try:yield active()
    finally:_scope.reset(token)

def capture(argv,*,limit=JSON_LIMIT,timeout=15,kind=None,locator=None,cwd=None,input_bytes=None):
    budget=active()
    if budget is None:raise ValueError('bounded observation scope required')
    deadline=time.monotonic()+min(timeout,budget.remaining())
    process=subprocess.Popen(argv,cwd=cwd,stdin=subprocess.PIPE if input_bytes is not None else subprocess.DEVNULL,
        stdout=subprocess.PIPE,stderr=subprocess.PIPE)
    buffers={'stdout':bytearray(),'stderr':bytearray()}
    selector=selectors.DefaultSelector()
    try:
        if input_bytes is not None:
            if len(input_bytes)>65536:raise ValueError('observation subprocess input limit exceeded')
            os.set_blocking(process.stdin.fileno(),False)
            pending=memoryview(input_bytes)
            if pending:selector.register(process.stdin,selectors.EVENT_WRITE,'stdin')
            else:process.stdin.close()
        for name in buffers:selector.register(getattr(process,name),selectors.EVENT_READ,name)
        while selector.get_map():
            left=min(deadline-time.monotonic(),budget.remaining())
            if left<=0:raise ValueError('terminal observation call deadline exceeded')
            for key,_ in selector.select(min(left,0.1)):
                if key.data=='stdin':
                    try:written=os.write(key.fileobj.fileno(),pending[:65536])
                    except BlockingIOError:continue
                    except BrokenPipeError:written=len(pending)
                    pending=pending[written:]
                    if not pending:
                        selector.unregister(key.fileobj);key.fileobj.close()
                    continue
                chunk=os.read(key.fileobj.fileno(),65536)
                if not chunk:selector.unregister(key.fileobj);continue
                name=key.data;cap=limit if name=='stdout' else 65536
                budget.charge(chunk)
                if len(buffers[name])+len(chunk)>cap:
                    raise ValueError('terminal observation response byte limit exceeded')
                buffers[name].extend(chunk)
        left=min(deadline-time.monotonic(),budget.remaining())
        if left<=0:raise ValueError('terminal observation process deadline exceeded')
        process.wait(timeout=left)
        raw=bytes(buffers['stdout'])
        if process.returncode:
            raise ObservationError('terminal observation transport failed: '+str(locator or argv[0]),raw,bytes(buffers['stderr']),process.returncode)
        if kind:budget.retain(kind,locator,raw)
        return raw
    except subprocess.TimeoutExpired as exc:
        raise ValueError('terminal observation process deadline exceeded') from exc
    finally:
        selector.close()
        if process.poll() is None:process.kill()
        process.wait()
        for stream in (process.stdout,process.stderr):stream.close()
        if process.stdin is not None and not process.stdin.closed:process.stdin.close()

def api(endpoint):
    return load(capture(['gh','api',endpoint],kind='github_api',locator=endpoint))

def pages(endpoint,key=None):
    result=[];seen=set();expected=None
    for page in range(1,101):
        value=api(endpoint+(' &' if '?' in endpoint else '?').replace(' &','&')+f'per_page=100&page={page}')
        batch=value.get(key) if key and isinstance(value,dict) else value
        if not isinstance(batch,list):raise ValueError('observation discovery malformed pagination')
        if len(batch)>100:raise ValueError('observation discovery oversized page')
        if isinstance(value,dict) and 'total_count' in value:
            total=value['total_count']
            if type(total) is not int or total<0 or total>10000 or (expected is not None and expected!=total):
                raise ValueError('observation pagination total inconsistent')
            expected=total
        for item in batch:
            if not isinstance(item,dict):raise ValueError('observation discovery malformed row')
            ident=item.get('id')
            if type(ident) is not int or ident<1 or ident in seen:raise ValueError('observation discovery invalid or duplicate identity')
            seen.add(ident);result.append(item)
        if len(batch)<100:
            if expected is not None and len(result)!=expected:raise ValueError('observation discovery incomplete pagination')
            return result
    raise ValueError('observation pagination bound exhausted')

def git(root,*args):
    object_locator=args[1] if len(args)==2 and args[0]=='show' and ':' in args[1] else None
    return capture(['git','-C',str(root),*args],limit=ARTIFACT_LIMIT,timeout=15,
        kind='git_object' if object_locator else None,locator=object_locator)

def artifact(repository,ident,member):
    positive(ident,'artifact')
    endpoint=f'repos/{repository}/actions/artifacts/{ident}/zip'
    raw=capture(['gh','api',endpoint],limit=ARTIFACT_LIMIT,kind='repository_artifact',locator=endpoint)
    try:
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            infos=archive.infolist()
            if len(infos)!=1 or infos[0].filename!=member or infos[0].file_size>ARTIFACT_LIMIT:
                raise ValueError('observation artifact archive members/size mismatch')
            if infos[0].flag_bits&1:raise ValueError('observation encrypted artifact unsupported')
            data=archive.read(infos[0])
    except (zipfile.BadZipFile,RuntimeError) as exc:raise ValueError('observation artifact archive invalid') from exc
    active().charge(data)
    return load(data),data
