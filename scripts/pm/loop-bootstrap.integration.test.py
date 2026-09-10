#!/usr/bin/env python3
"""Real shell + temporary Git + fake GitHub manual bootstrap/resume acceptance."""
import json
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
UID = 'task_' + '1' * 32
FAKE = r'''#!/usr/bin/env python3
import json, os, pathlib, sys
a=sys.argv[1:]; p=pathlib.Path(os.environ['FAKE_GH_STATE'])
s=json.loads(p.read_text()) if p.exists() else {'creates':0,'fields':{}}
def emit(v):
 p.write_text(json.dumps(s)); print(json.dumps(v) if not isinstance(v,str) else v)
def val(k): return a[a.index(k)+1]
url='https://github.com/eng-cc/oasis7/issues/1'
if a[:2]==['issue','list']:
 if s.get('lost') and os.environ.get('FAKE_SEARCH')=='empty': emit([])
 elif s.get('lost') and os.environ.get('FAKE_SEARCH')=='multiple': emit([{'number':1},{'number':2}])
 elif s.get('lost') and os.environ.get('FAKE_SEARCH')=='limit': emit([{'number':n} for n in range(1,6)])
 elif s.get('lost') and os.environ.get('FAKE_SEARCH')=='error': raise SystemExit('read unavailable')
 else: emit([{'number':1,'url':url,'title':'[PM] fixture','state':'OPEN'}] if s.get('body') else [])
elif a[:2] in (['issue','create'],['issue','edit']):
 s['body']=pathlib.Path(val('--body-file')).read_text()
 if a[1]=='create': s['creates']+=1
 if a[1]=='create' and os.environ.get('FAKE_LOSS') and not s.get('lost'):
  s['lost']=True; p.write_text(json.dumps(s)); raise SystemExit('created but response lost')
 if a[1]=='edit' and '- status: `committed`' in s['body'] and os.environ.get('FAKE_LIFECYCLE')=='move-task':
  p.write_text(json.dumps(s)); raise SystemExit('move succeeded but response lost')
 emit(url)
elif a[:2]==['issue','view']: emit({'number':1,'url':url,'title':'[PM] fixture','state':'OPEN','stateReason':None,'body':s['body']})
elif a[:2]==['issue','comment']:
 if 'Evidence Phase: start\n' in pathlib.Path(val('--body-file')).read_text() and os.environ.get('FAKE_LIFECYCLE')=='workflow-report-uncertain':
  raise SystemExit('start outcome unavailable')
 comments=s.setdefault('comments',[]); comments.append({'id':len(comments)+1,'body':pathlib.Path(val('--body-file')).read_text()})
 if 'Evidence Phase: start\n' in comments[-1]['body'] and os.environ.get('FAKE_LIFECYCLE')=='workflow-report':
  p.write_text(json.dumps(s)); raise SystemExit('start succeeded but response lost')
 emit(url+'#issuecomment-'+str(len(comments)))
elif a[:1]==['api'] and a[1].startswith('repos/eng-cc/oasis7/issues?'):
 emit([{'id':1,'number':1,'body':s['body']}] if s.get('body') else [])
elif a[:2]==['api','repos/eng-cc/oasis7/issues/1/comments']: emit([s.get('comments',[])])
elif a[:2]==['api','repos/eng-cc/oasis7/issues/1']: emit({'number':1,'body':s['body'],'url':url})
elif a[:1]==['api'] and a[1].startswith('repos/eng-cc/oasis7/issues/comments/'): emit(s['comments'][int(a[1].rsplit('/',1)[1])-1])
elif a[:2]==['project','view']: emit({'id':'P','number':1})
elif a[:2]==['project','item-add']: emit({'id':'I'})
elif a[:2]==['project','field-list']:
 choices={'Status':['Todo','In Progress'],'PM Status':['candidate','committed'],'Workflow Phase':['bootstrap','execution'], 'Owner Role':['repository_health_engineer'], 'Module':['engineering'],'Priority':['P2'],'Test Tier Required':['n/a'],'Loop':['product','system','code']}
 fields=[{'id':n,'name':n,'type':'ProjectV2SingleSelectField','options':[{'id':v,'name':v} for v in values]} for n,values in choices.items()]
 fields += [{'id':n,'name':n,'type':'ProjectV2Field'} for n in ['Task UID','Canonical Worktree','Change ID','Blocked Reason','PR','Last PM Update']]
 emit({'fields':fields})
elif a[:2]==['project','item-edit']:
 s['fields'][val('--field-id')]=val('--text') if '--text' in a else val('--single-select-option-id'); emit({})
elif a[:2]==['api','graphql']:
 node={'id':'I','project':{'id':'P','number':1,'owner':{'login':'eng-cc'}},'fieldValues':{'pageInfo':{'hasNextPage':False},'nodes':[{'text':v,'field':{'name':k}} for k,v in s['fields'].items()]}}
 emit({'data':{'nodes':[node]}})
else: raise SystemExit('unsupported fake gh '+repr(a))
'''

class BootstrapEndToEnd(unittest.TestCase):
    def test_uncertain_start_never_reposts(self):
        self.test_full_flags_and_explicit_resume_reuse_task(lifecycle='workflow-report-uncertain')

    def test_created_task_wrong_identity_does_not_resume(self):
        self.test_full_flags_and_explicit_resume_reuse_task(lifecycle='new-task', identity_drift=True)

    def test_created_task_retry_completes_missing_bootstrap_stages(self):
        for point in ('new-task', 'move-task', 'workflow-report'):
            with self.subTest(point=point):
                self.test_full_flags_and_explicit_resume_reuse_task(lifecycle=point)

    def test_interrupted_setup_reconciles_missing_artifacts(self):
        self.test_full_flags_and_explicit_resume_reuse_task(setup='missing')

    def test_interrupted_setup_preserves_custom_config(self):
        self.test_full_flags_and_explicit_resume_reuse_task(setup='config_only')

    def test_interrupted_setup_preserves_complete_artifacts(self):
        self.test_full_flags_and_explicit_resume_reuse_task(setup='complete')

    def test_interrupted_setup_rejects_invalid_target(self):
        self.test_full_flags_and_explicit_resume_reuse_task(setup='invalid_target')

    def test_interrupted_setup_rejects_invalid_config(self):
        self.test_full_flags_and_explicit_resume_reuse_task(setup='invalid_config')

    def test_pinned_tools_with_newer_task_base(self):
        self.test_full_flags_and_explicit_resume_reuse_task(advanced=True)

    def test_uncertain_create_never_reposts(self):
        self.test_full_flags_and_explicit_resume_reuse_task(loss=True)

    def test_full_flags_and_explicit_resume_reuse_task(self, advanced=False, loss=False, setup=None, lifecycle=None, identity_drift=False):
        with tempfile.TemporaryDirectory() as directory:
            temp = Path(directory)
            root = temp / 'repo'
            (root / 'scripts').mkdir(parents=True)
            shutil.copytree(ROOT / 'scripts/pm', root / 'scripts/pm', ignore=shutil.ignore_patterns('__pycache__'))
            for name in ['new-task-worktree.sh', 'worktree-harness-lib.sh']:
                shutil.copy2(ROOT / 'scripts' / name, root / 'scripts' / name)
            (root / '.gitignore').write_text('.pm/\ntarget\nconfig.toml\n__pycache__/\n')
            (root / 'config.toml').write_text('canonical = true\n')
            cargo = root / 'scripts/cargo-dev.sh'
            cargo.write_text('#!/bin/sh\nprintf "%s\\n" "$TEST_SHARED_TARGET"\n')
            cargo.chmod(0o755)
            def git(*args): return subprocess.check_output(['git','-C',str(root),*args],text=True).strip()
            git('init','-q','-b','main'); git('config','user.email','test@example.invalid'); git('config','user.name','Test')
            git('add','.'); git('commit','-qm','fixture')
            origin = temp / 'origin.git'
            subprocess.run(['git','init','--bare','-q',str(origin)],check=True)
            git('remote','add','origin','https://github.com/eng-cc/oasis7.git')
            git('config',f'url.{origin}.insteadOf','https://github.com/eng-cc/oasis7.git')
            git('push','-q','origin','main'); git('symbolic-ref','refs/remotes/origin/HEAD','refs/remotes/origin/main')
            subprocess.run(['git','--git-dir',str(origin),'symbolic-ref','HEAD','refs/heads/main'],check=True)
            base = git('rev-parse','HEAD')
            binary = temp / 'bin'; binary.mkdir()
            gh = binary / 'gh'; gh.write_text(FAKE); gh.chmod(0o755)
            state = temp / 'github.json'
            env = dict(os.environ, PATH=str(binary)+os.pathsep+os.environ['PATH'], FAKE_GH_STATE=str(state),
                       TEST_SHARED_TARGET=str(temp/'target'), PYTHONDONTWRITEBYTECODE='1')
            binding = dict(schema='oasis7.loop-task/v1',task_uid=UID,change_id='change-fixture',loop='code',owner_role='repository_health_engineer',bootstrap_epoch=1,
                manual_request_ref='message:1',request_key='request:1',write_scope=['scripts/**'],out_of_scope=[],input_contracts=[],acceptance_refs=['M06'],dependencies=[],
                target_delivery='fixture',policy_digest='sha256:'+hashlib.sha256((root/'scripts/pm/loop-policy.v1.json').read_bytes()).hexdigest(),policy_commit=base)
            source = temp / 'binding.json'; source.write_text(json.dumps(binding))
            start = root
            expected_base = base
            if advanced:
                start = temp / 'pinned-tools'
                git('worktree','add','--detach',str(start),base)
                (root/'later.txt').write_text('new main base\n')
                for helper in ['new-task.sh', 'move-task.sh', 'workflow-report.sh', 'bootstrap-task-snapshot.py']:
                    (root/'scripts/pm'/helper).write_text('#!/bin/sh\necho candidate-helper-executed >&2\nexit 91\n')
                git('add','.'); git('commit','-qm','candidate base'); git('push','-q','origin','main')
                expected_base = git('rev-parse','HEAD')
            target = temp / 'task'
            command = ['bash','scripts/new-task-worktree.sh','engineering','loop-test','--path',str(target),'--branch','codex/loop-test',
                '--pm-owner-role','repository_health_engineer','--pm-title','fixture','--pm-source-ref','fixture','--pm-acceptance','M06',
                '--pm-loop','code','--pm-loop-binding',str(source),'--pm-request-key','request:1','--pm-manual-request-ref','message:1','--json']
            if loss:
                env['FAKE_LOSS']='1'
            if lifecycle:
                env['FAKE_LIFECYCLE']=lifecycle
                real_python = shutil.which('python3')
                wrapper = binary/'python3'
                wrapper.write_text('#!/bin/sh\n"'+real_python+'" "$@"\nstatus=$?\nif [ "$status" = 0 ] && [ "$2" = "'+lifecycle+'" ]; then exit 97; fi\nexit "$status"\n')
                wrapper.chmod(0o755)
                failed = subprocess.run(command,cwd=start,env=env,text=True,capture_output=True)
                self.assertNotEqual(failed.returncode,0,failed.stdout)
                self.assertEqual(json.loads(state.read_text())['creates'],1)
                self.assertFalse((target/'.pm/scratch'/UID/'bootstrap-task-snapshot.json').exists())
                wrapper.unlink()
                env.pop('FAKE_LIFECYCLE')
                if identity_drift:
                    value=json.loads(state.read_text())
                    value['body']=value['body'].replace('task_uid: '+UID,'task_uid: task_'+'2'*32)
                    state.write_text(json.dumps(value))
                before_retry=json.loads(state.read_text())
            if setup:
                # Fail after the actual worktree add side effect, before setup.
                real_git = shutil.which('git')
                wrapper = binary/'git'
                wrapper.write_text('#!/bin/sh\n"'+real_git+'" "$@"\nstatus=$?\nif [ "$1 $2" = "worktree add" ] && [ "$status" = 0 ]; then exit 97; fi\nexit "$status"\n')
                wrapper.chmod(0o755)
                interrupted = subprocess.run(command,cwd=start,env=env,text=True,capture_output=True)
                self.assertNotEqual(interrupted.returncode,0)
                self.assertTrue((target/'.git').is_file(),interrupted.stderr)
                self.assertFalse(state.exists())
                wrapper.unlink()
                if setup in ('config_only','complete'):
                    (target/'config.toml').write_text('custom = true\n')
                if setup == 'complete':
                    (temp/'target').mkdir(); (target/'target').symlink_to(temp/'target')
                if setup == 'invalid_target': (target/'target').mkdir()
                if setup == 'invalid_config': (target/'config.toml').mkdir()
            result = subprocess.run(command,cwd=start,env=env,text=True,capture_output=True)
            if lifecycle == 'workflow-report-uncertain' or identity_drift:
                self.assertNotEqual(result.returncode,0,result.stdout)
                if not identity_drift: self.assertIn('start outcome uncertain',result.stderr)
                self.assertEqual(json.loads(state.read_text()),before_retry)
                self.assertFalse((target/'.pm/scratch'/UID/'bootstrap-task-snapshot.json').exists())
                return
            if setup:
                if setup.startswith('invalid_'):
                    self.assertNotEqual(result.returncode,0,result.stdout)
                    self.assertTrue((target/'.git').is_file())
                    self.assertTrue((target/('target' if setup == 'invalid_target' else 'config.toml')).is_dir())
                    self.assertFalse(state.exists())
                else:
                    self.assertEqual(result.returncode,0,result.stderr)
                    self.assertEqual((target/'config.toml').read_text(),'canonical = true\n' if setup == 'missing' else 'custom = true\n')
                    self.assertTrue((target/'target').is_symlink())
                    self.assertEqual((target/'target').resolve(),(temp/'target').resolve())
                    self.assertEqual(json.loads(state.read_text())['creates'],1)
                return
            if loss:
                self.assertNotEqual(result.returncode,0,result.stdout)
                self.assertEqual(json.loads(state.read_text())['creates'],1)
                for mode in ['empty','multiple','limit','error']:
                    retry = subprocess.run(command,cwd=start,env={**env,'FAKE_SEARCH':mode},text=True,capture_output=True)
                    self.assertNotEqual(retry.returncode,0,retry.stdout)
                    self.assertEqual(json.loads(state.read_text())['creates'],1,mode+retry.stderr)
                recovered = subprocess.run(command,cwd=start,env=env,text=True,capture_output=True)
                self.assertEqual(recovered.returncode,0,recovered.stderr)
                self.assertEqual(json.loads(state.read_text())['creates'],1)
                return
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertEqual(json.loads(state.read_text())['creates'],1)
            snapshot = json.loads((target/'.pm/scratch'/UID/'bootstrap-task-snapshot.json').read_text())
            self.assertEqual(snapshot['git']['base']['oid'],expected_base)
            self.assertEqual(snapshot['task']['loop_binding'],binding)
            if lifecycle:
                before=json.loads(state.read_text())
                starts=[c for c in before.get('comments',[]) if 'Evidence Phase: start\n' in c['body']]
                self.assertEqual(len(starts),1)
                repeated=subprocess.run(command,cwd=start,env=env,text=True,capture_output=True)
                self.assertEqual(repeated.returncode,0,repeated.stderr)
                after=json.loads(state.read_text())
                self.assertEqual(after['creates'],1)
                self.assertEqual(after['comments'],before['comments'])
                self.assertEqual(after['fields']['PM Status'],'committed')
                self.assertEqual(after['fields']['Workflow Phase'],'execution')
                return
            if advanced:
                return
            # A later remote default head must not silently rebase the same request.
            (root/'later.txt').write_text('unrelated upstream change\n')
            git('add','later.txt'); git('commit','-qm','later'); git('push','-q','origin','main')
            retry = subprocess.run(command,cwd=target,env=env,text=True,capture_output=True)
            self.assertEqual(retry.returncode,0,retry.stderr)
            self.assertEqual(json.loads(retry.stdout)['pm']['task_uid'],UID)
            resume = subprocess.run(['bash','scripts/new-task-worktree.sh','--pm-task-uid',UID,'--pm-loop','code','--pm-manual-request-ref','message:2'],cwd=target,env=env,text=True,capture_output=True)
            self.assertEqual(resume.returncode,0,resume.stderr)
            self.assertEqual(json.loads(resume.stdout)['pm']['task_uid'],UID)
            self.assertEqual(json.loads(state.read_text())['creates'],1)
            self.assertEqual(json.loads(state.read_text())['fields']['Loop'],'code')
            revised = dict(binding, bootstrap_epoch=2, write_scope=['scripts/pm/**'])
            source.write_text(json.dumps(revised))
            bind_command = ['python3',str(target/'scripts/pm/github-project-task.py'),'bind-loop',str(target),
                            '--task-uid',UID,'--loop-binding',str(source),'--manual-request-ref','message:3','--json']
            denied = subprocess.run(bind_command,cwd=root,env=env,text=True,capture_output=True)
            self.assertNotEqual(denied.returncode,0)
            migrated = subprocess.run(bind_command+['--migrate-epoch','2'],cwd=root,env=env,text=True,capture_output=True)
            self.assertEqual(migrated.returncode,0,migrated.stderr)
            snapshot = json.loads((target/'.pm/scratch'/UID/'bootstrap-task-snapshot.json').read_text())
            self.assertEqual(snapshot['task']['bootstrap_epoch'],2)
            self.assertTrue((target/'.pm/scratch'/UID/'bootstrap-task-snapshot.epoch-1.json').exists())
            trusted = temp / 'trusted'
            git('worktree','add','--detach',str(trusted),base)
            facade = ['python3',str(trusted/'scripts/pm/loop.py'),'bind','--repo-root',str(target),
                      '--tool-root',str(trusted),'--task-uid',UID,'--loop-binding',str(source),
                      '--manual-request-ref',revised['manual_request_ref'],'--json']
            # Actual adapter preflight rejection must leave no mutation intent.
            preflight_adapter = "import sys,importlib.util; from pathlib import Path; sys.path.insert(0,str(Path(sys.argv[1]).parent)); spec=importlib.util.spec_from_file_location('pm',sys.argv[1]); pm=importlib.util.module_from_spec(spec); sys.modules['pm']=pm; spec.loader.exec_module(pm)\ndef reject(*a,**kw): raise SystemExit('preflight no write')\npm.validate_loop_inputs=reject; raise SystemExit(pm.main(sys.argv[2:]))"
            preflight_facade = "import sys,subprocess; sys.path.insert(0,sys.argv[1]); import loop; original=subprocess.check_output; injected=sys.argv[2]\ndef fault(args,*a,**kw):\n if 'bind-loop' in args: args=[sys.executable,'-c',injected,args[1],*args[2:]]\n return original(args,*a,**kw)\nsubprocess.check_output=fault; sys.argv=sys.argv[3:]; raise SystemExit(loop.main())"
            common = Path(git('rev-parse','--path-format=absolute','--git-common-dir'))
            before_state = state.read_bytes()
            denied = subprocess.run(['python3','-c',preflight_facade,str(trusted/'scripts/pm'),preflight_adapter,*facade[1:]],cwd=trusted,env=env,text=True,capture_output=True)
            self.assertNotEqual(denied.returncode,0)
            self.assertEqual(state.read_bytes(),before_state)
            self.assertEqual(list((common/'oasis7-loop-recovery').glob('*.actions.jsonl')),[])
            # Run the real adapter to completion, then lose only its response to
            # the facade. Production code and all readbacks remain unchanged.
            inject = "import sys,subprocess; sys.path.insert(0,sys.argv[1]); import loop; original=subprocess.check_output\ndef lost(args,*a,**kw):\n result=original(args,*a,**kw)\n if 'bind-loop' in args: raise subprocess.CalledProcessError(1,args)\n return result\nsubprocess.check_output=lost; sys.argv=sys.argv[2:]; raise SystemExit(loop.main())"
            lost = subprocess.run(['python3','-c',inject,str(trusted/'scripts/pm'),*facade[1:]],cwd=trusted,env=env,text=True,capture_output=True)
            self.assertNotEqual(lost.returncode,0,lost.stdout+lost.stderr)
            recover = subprocess.run(['python3',str(trusted/'scripts/pm/loop.py'),'recover','--repo-root',str(target),'--tool-root',str(trusted),'--task-uid',UID,'--manual-request-ref','message:recover','--json'],cwd=trusted,env=env,text=True,capture_output=True)
            recovered = json.loads(recover.stdout)
            self.assertEqual(recovered.get('pending_actions'),[],recover.stdout+recover.stderr)
            bound = subprocess.run(facade,cwd=trusted,env=env,text=True,capture_output=True)
            self.assertEqual(bound.returncode,0,bound.stdout+bound.stderr)
            self.assertEqual(json.loads(bound.stdout)['status'],'bound')
            common = Path(git('rev-parse','--path-format=absolute','--git-common-dir'))
            journals = list((common/'oasis7-loop-recovery').glob('*.actions.jsonl'))
            self.assertEqual(len(journals),1)
            events = [json.loads(line) for line in journals[0].read_text().splitlines()]
            self.assertEqual(events[0]['kind'],'bind_loop')
            self.assertTrue(events[-1]['reconciled'])
            again = subprocess.run(facade,cwd=trusted,env=env,text=True,capture_output=True)
            self.assertEqual(again.returncode,0,again.stdout+again.stderr)
            self.assertEqual(json.loads(state.read_text())['creates'],1)
            # Keep the actual adapter alive after killing its facade parent.
            # The inherited flock must exclude recovery without a second action.
            import time
            import signal
            pidfile = temp/'bind-child.pid'; release = temp/'bind-child.release'
            pause_adapter = "import sys,os,time,importlib.util; from pathlib import Path; sys.path.insert(0,str(Path(sys.argv[1]).parent)); spec=importlib.util.spec_from_file_location('pm',sys.argv[1]); pm=importlib.util.module_from_spec(spec); sys.modules['pm']=pm; spec.loader.exec_module(pm); original=pm.update_project_fields\ndef pause(*a,**kw):\n Path(os.environ['BIND_PID']).write_text(str(os.getpid()))\n while not Path(os.environ['BIND_RELEASE']).exists(): time.sleep(.02)\n return original(*a,**kw)\npm.update_project_fields=pause; raise SystemExit(pm.main(sys.argv[2:]))"
            parent = subprocess.Popen(['python3','-c',preflight_facade,str(trusted/'scripts/pm'),pause_adapter,*facade[1:]],cwd=trusted,env=dict(env,BIND_PID=str(pidfile),BIND_RELEASE=str(release)),text=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            child = None
            try:
                for _ in range(500):
                    if pidfile.exists(): break
                    if parent.poll() is not None: self.fail('bind adapter exited before pause')
                    time.sleep(.02)
                self.assertTrue(pidfile.exists())
                child = int(pidfile.read_text()); parent.kill(); parent.wait(); os.kill(child,0)
                recovery_command = ['python3',str(trusted/'scripts/pm/loop.py'),'recover','--repo-root',str(target),'--tool-root',str(trusted),'--task-uid',UID,'--manual-request-ref','message:recover-child','--json']
                denied = subprocess.run(recovery_command,cwd=trusted,env=env,text=True,capture_output=True)
                self.assertNotEqual(denied.returncode,0)
                self.assertIn('active OS lock',denied.stdout+denied.stderr)
                events = [json.loads(line) for line in journals[0].read_text().splitlines()]
                self.assertEqual(events[-1]['kind'],'bind_loop')
                self.assertFalse(events[-1].get('reconciled',False))
                release.touch()
                for _ in range(500):
                    recovered = subprocess.run(recovery_command,cwd=trusted,env=env,text=True,capture_output=True)
                    if 'active OS lock' not in recovered.stdout+recovered.stderr: break
                    time.sleep(.02)
                self.assertEqual(recovered.returncode,0,recovered.stdout+recovered.stderr)
                self.assertEqual(json.loads(recovered.stdout).get('pending_actions'),[])
            finally:
                release.touch()
                if parent.poll() is None: parent.kill(); parent.wait()
                if child:
                    try: os.kill(child,signal.SIGTERM)
                    except ProcessLookupError: pass
            local_gate = ['python3',str(trusted/'scripts/pm/loop-local-gate.py'),'--root',str(target),'--task-uid',UID,'--base',base,'--head',base,'--tool-root',str(trusted),'--json']
            admitted = subprocess.run(local_gate,cwd=trusted,env=env,text=True,capture_output=True)
            self.assertEqual(admitted.returncode,0,admitted.stdout+admitted.stderr)
            stale = local_gate.copy(); stale[stale.index('--head')+1]='0'*40
            rejected = subprocess.run(stale,cwd=trusted,env=env,text=True,capture_output=True)
            self.assertNotEqual(rejected.returncode,0)
            # Interrupt the real binding adapter immediately after each durable
            # transition. Only the documented facade recover command may repair.
            adapter_inject = "import sys,importlib.util; from pathlib import Path; sys.path.insert(0,str(Path(sys.argv[1]).parent)); spec=importlib.util.spec_from_file_location('pm',sys.argv[1]); pm=importlib.util.module_from_spec(spec); sys.modules['pm']=pm; spec.loader.exec_module(pm); name=sys.argv[2]; original=getattr(pm,name)\ndef stop(*a,**kw):\n result=original(*a,**kw)\n raise SystemExit(91)\nsetattr(pm,name,stop); raise SystemExit(pm.main(sys.argv[3:]))"
            facade_inject = "import sys,subprocess; sys.path.insert(0,sys.argv[1]); import loop; original=subprocess.check_output; injected=sys.argv[2]; point=sys.argv[3]\ndef interrupt(args,*a,**kw):\n if 'bind-loop' in args: args=[sys.executable,'-c',injected,args[1],point,*args[2:]]\n return original(args,*a,**kw)\nsubprocess.check_output=interrupt; sys.argv=sys.argv[4:]; raise SystemExit(loop.main())"
            for epoch, point in enumerate(('update_issue_body', 'update_project_fields', 'merge_task_mapping'), 3):
                revised = dict(revised, bootstrap_epoch=epoch, manual_request_ref='message:epoch-' + str(epoch))
                source.write_text(json.dumps(revised))
                migrate = facade.copy()
                migrate[migrate.index('--manual-request-ref')+1] = revised['manual_request_ref']
                migrate += ['--migrate-epoch', str(epoch)]
                failed = subprocess.run(['python3','-c',facade_inject,str(trusted/'scripts/pm'),adapter_inject,point,*migrate[1:]],cwd=trusted,env=env,text=True,capture_output=True)
                self.assertNotEqual(failed.returncode,0,failed.stdout+failed.stderr)
                recovered = subprocess.run(['python3',str(trusted/'scripts/pm/loop.py'),'recover','--repo-root',str(target),'--tool-root',str(trusted),'--task-uid',UID,'--manual-request-ref','message:recover-'+str(epoch),'--json'],cwd=trusted,env=env,text=True,capture_output=True)
                payload = json.loads(recovered.stdout)
                self.assertEqual(payload.get('pending_actions'),[],recovered.stdout+recovered.stderr)
                mapping = json.loads((target/'.pm/github-project-sync/tasks.json').read_text())['tasks'][UID]
                snapshot = json.loads((target/'.pm/scratch'/UID/'bootstrap-task-snapshot.json').read_text())
                lineage = json.loads((common/'oasis7-loop-lineage'/ (UID+'.json')).read_text())
                self.assertEqual(mapping['loop_binding'],revised)
                self.assertEqual(snapshot['task']['loop_binding'],revised)
                self.assertEqual(lineage['loop_binding'],revised)
                self.assertEqual(json.loads(state.read_text())['creates'],1)

if __name__=='__main__': unittest.main()
