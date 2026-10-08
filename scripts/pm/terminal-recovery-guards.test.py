#!/usr/bin/env python3
"""Offline real-process guards; simulated transport is never validator authority."""
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import unittest
from unittest import mock
import zipfile

spec=importlib.util.spec_from_file_location('recovery_observation_guards',Path(__file__).with_name('recovery_observation.py'))
obs=importlib.util.module_from_spec(spec);spec.loader.exec_module(obs)

class ObservationGuards(unittest.TestCase):
    def child(self,script,check):
        real=subprocess.Popen;children=[]
        def launch(argv,**kwargs):
            self.assertEqual(argv,['gh','api','offline/guard'])
            proc=real([sys.executable,'-S','-c',script],**kwargs);children.append(proc);return proc
        with mock.patch.object(obs.subprocess,'Popen',side_effect=launch):
            with obs.observation() as budget:check(budget)
        self.assertEqual(len(children),1)
        proc=children[0]
        self.assertIsNotNone(proc.returncode)
        self.assertTrue(proc.stdout.closed and proc.stderr.closed)
        with self.assertRaises(ProcessLookupError):os.kill(proc.pid,0)
        return proc

    def test_stdout_overflow_kills_reaps_without_retained_evidence(self):
        def check(b):
            with self.assertRaisesRegex(ValueError,'response byte limit'):
                obs.capture(['gh','api','offline/guard'],kind='github_api',locator='offline/guard')
            self.assertEqual(b.evidence,[])
        p=self.child("import os,time;os.write(1,b'x'*(10*1024*1024+1));time.sleep(60)",check)
        self.assertLess(p.returncode,0)

    def test_stderr_overflow_kills_reaps(self):
        def check(b):
            with self.assertRaisesRegex(ValueError,'response byte limit'):
                obs.capture(['gh','api','offline/guard'])
        p=self.child("import os,time;os.write(2,b'x'*65537);time.sleep(60)",check)
        self.assertLess(p.returncode,0)

    def test_exact_stdout_boundary_and_exit_drains_output(self):
        def check(b):
            raw=obs.capture(['gh','api','offline/guard'])
            self.assertEqual(len(raw),obs.JSON_LIMIT);self.assertEqual(b.bytes,obs.JSON_LIMIT)
        self.child("import os;os.write(1,b'x'*(10*1024*1024))",check)

    def test_closed_pipes_live_child_obeys_default_deadline_and_reaps(self):
        def check(b):
            with self.assertRaisesRegex(ValueError,'deadline'):
                obs.capture(['gh','api','offline/guard'])
        p=self.child("import os,time;os.close(1);os.close(2);time.sleep(60)",check)
        self.assertLess(p.returncode,0)

    def test_nested_scope_cannot_reset_aggregate_bytes_or_deadline(self):
        with obs.observation() as first:
            first.charge(b'x'*obs.TOTAL_LIMIT)
            with obs.observation() as second:
                self.assertIs(first,second)
                with self.assertRaisesRegex(ValueError,'aggregate byte limit'):second.charge(b'x')
            deadline=first.deadline
            with mock.patch.object(obs.time,'monotonic',return_value=deadline):
                with self.assertRaisesRegex(ValueError,'aggregate deadline'):
                    with obs.observation():pass
        self.assertIsNone(obs.active())
        with obs.observation() as fresh:self.assertEqual(fresh.bytes,0)

    def test_nonzero_and_empty_json_are_not_success(self):
        def check(b):
            with self.assertRaisesRegex(ValueError,'transport failed'):obs.api('offline/guard')
        self.child("import sys;sys.stderr.write('failure');sys.exit(7)",check)
        self.child('pass',lambda b:self.assertRaises(ValueError,obs.api,'offline/guard'))

    def test_inflated_artifact_member_rejected_before_retention(self):
        stream=io.BytesIO()
        with zipfile.ZipFile(stream,'w',zipfile.ZIP_DEFLATED) as z:z.writestr('proof.json',b'x'*(obs.ARTIFACT_LIMIT+1))
        real=subprocess.Popen;blob=stream.getvalue();children=[]
        def launch(argv,**kwargs):
            self.assertEqual(argv,['gh','api','repos/eng-cc/oasis7/actions/artifacts/1/zip'])
            import base64
            code='import base64,os;os.write(1,base64.b64decode('+repr(base64.b64encode(blob).decode())+'))'
            p=real([sys.executable,'-S','-c',code],**kwargs);children.append(p);return p
        with mock.patch.object(obs.subprocess,'Popen',side_effect=launch),obs.observation() as b:
            with self.assertRaisesRegex(ValueError,'members/size'):obs.artifact('eng-cc/oasis7',1,'proof.json')
            self.assertEqual(b.bytes,len(blob))
        self.assertEqual(children[0].returncode,0);self.assertTrue(children[0].stdout.closed)

    def test_pagination_completion_overlap_and_bound(self):
        def page(n):return [{'id':n*100+i+1} for i in range(100)]
        with mock.patch.object(obs,'api',side_effect=[page(0),[]]) as api:
            self.assertEqual(len(obs.pages('offline/list')),100)
            self.assertEqual(api.call_args_list[-1].args[0],'offline/list?per_page=100&page=2')
        with mock.patch.object(obs,'api',side_effect=[page(0),[{'id':1}]]):
            with self.assertRaisesRegex(ValueError,'duplicate'):obs.pages('offline/list')
        with mock.patch.object(obs,'api',return_value={'total_count':2,'items':[{'id':1}]}):
            with self.assertRaisesRegex(ValueError,'incomplete'):obs.pages('offline/list',key='items')
        with mock.patch.object(obs,'api',side_effect=[page(n) for n in range(100)]) as api:
            with self.assertRaisesRegex(ValueError,'bound exhausted'):obs.pages('offline/list')
            self.assertEqual(api.call_count,100)

class ExtendedObservationGuards(unittest.TestCase):
    def test_nonreading_stdin_respects_default_deadline_with_outer_cleanup(self):
        import tempfile,signal
        with tempfile.TemporaryDirectory() as temp:
            pidfile=Path(temp)/'child.pid'
            module=str(Path(__file__).with_name('recovery_observation.py').resolve())
            code="""import importlib.util,subprocess,sys,os
s=importlib.util.spec_from_file_location('guardleaf',sys.argv[1]);m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
real=subprocess.Popen;children=[]
def launch(argv,**kwargs):
 assert argv==['gh','api','offline/stdin']
 p=real([sys.executable,'-S','-c','import time;time.sleep(60)'],**kwargs)
 children.append(p);open(sys.argv[2],'w').write(str(p.pid));return p
m.subprocess.Popen=launch
try:
 with m.observation():m.capture(['gh','api','offline/stdin'],input_bytes=b'x'*65536)
 raise AssertionError('nonreading stdin unexpectedly succeeded')
except ValueError as e:
 assert 'deadline' in str(e),str(e)
 assert children[0].returncode is not None
 assert children[0].stdin.closed and children[0].stdout.closed and children[0].stderr.closed
 try:os.kill(children[0].pid,0)
 except ProcessLookupError:pass
 else:raise AssertionError('child not reaped')
 print('deadline refused; real child killed/reaped')
"""
            runner=subprocess.Popen([sys.executable,'-S','-c',code,module,str(pidfile)],stdout=subprocess.PIPE,stderr=subprocess.PIPE,start_new_session=True)
            try:
                try:out,err=runner.communicate(timeout=20)
                except subprocess.TimeoutExpired:
                    self.fail('capture blocked on synchronous stdin beyond default15s; outer20s cleanup required')
                self.assertEqual(runner.returncode,0,(out,err))
                self.assertIn(b'killed/reaped',out)
            finally:
                if runner.poll() is None:
                    os.killpg(runner.pid,signal.SIGKILL);runner.communicate()
                runner.stdout.close();runner.stderr.close()

    def test_nonpositive_ids_and_oversized_page_reject(self):
        for batch in ([{'id':0}],[{'id':-1}],[{'id':i+1} for i in range(101)]):
            with self.subTest(batch_size=len(batch),first_id=batch[0]['id']),mock.patch.object(obs,'api',side_effect=[batch,[]]) as api:
                with self.assertRaises(ValueError):obs.pages('offline/malformed')
                self.assertEqual(api.call_count,1)

    def test_artifact_capture_exact_compressed_limit_and_plus_one(self):
        real=subprocess.Popen
        for size in (obs.ARTIFACT_LIMIT,obs.ARTIFACT_LIMIT+1):
            with self.subTest(size=size):
                children=[]
                def launch(argv,**kwargs):
                    self.assertEqual(argv,['gh','api','repos/eng-cc/oasis7/actions/artifacts/1/zip'])
                    code="import os,time;os.write(1,b'x'*"+str(size)+");time.sleep(60)" if size>obs.ARTIFACT_LIMIT else "import os;os.write(1,b'x'*"+str(size)+")"
                    p=real([sys.executable,'-S','-c',code],**kwargs);children.append(p);return p
                with mock.patch.object(obs.subprocess,'Popen',side_effect=launch),obs.observation() as b:
                    with self.assertRaisesRegex(ValueError,'byte limit' if size>obs.ARTIFACT_LIMIT else 'archive invalid'):
                        obs.artifact('eng-cc/oasis7',1,'proof.json')
                    self.assertEqual(len(b.evidence),0 if size>obs.ARTIFACT_LIMIT else 1)
                    if size==obs.ARTIFACT_LIMIT:self.assertEqual(b.bytes,size)
                child=children[0];self.assertIsNotNone(child.returncode)
                self.assertTrue(child.stdout.closed and child.stderr.closed)
                with self.assertRaises(ProcessLookupError):os.kill(child.pid,0)

    def test_real_review_nested_helper_intersects_outer_absolute_deadline(self):
        import time
        # Real leaf module identity is mandatory: hook imports must see this scope.
        realclock=time.monotonic;real=subprocess.Popen;children=[]
        spec=importlib.util.spec_from_file_location('guard_review_handoff',Path(__file__).with_name('review_preflight_handoff.py'))
        helper=importlib.util.module_from_spec(spec);spec.loader.exec_module(helper)
        def launch(argv,**kwargs):
            self.assertEqual(argv,['gh','api','offline/nested'])
            p=real([sys.executable,'-S','-c','import time;time.sleep(60)'],**kwargs);children.append(p);return p
        with mock.patch.dict(sys.modules,{'recovery_observation':obs}),obs.observation() as b:
            deadline=b.deadline
            with mock.patch.object(obs.time,'monotonic',side_effect=lambda:realclock()+299.5),mock.patch.object(obs.subprocess,'Popen',side_effect=launch):
                started=realclock()
                with self.assertRaisesRegex(ValueError,'deadline'):helper.gh_json(['offline/nested'],'nested actual reader')
                self.assertLess(realclock()-started,2)
                self.assertEqual(b.deadline,deadline);self.assertEqual(b.evidence,[])
        self.assertEqual(len(children),1);self.assertLess(children[0].returncode,0)
        self.assertTrue(children[0].stdout.closed and children[0].stderr.closed)
        with self.assertRaises(ProcessLookupError):os.kill(children[0].pid,0)

if __name__=='__main__':unittest.main(verbosity=2)
