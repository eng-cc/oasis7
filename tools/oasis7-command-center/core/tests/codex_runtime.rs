#![cfg(unix)]
use oasis7_command_center_core::{CommandCenter, codex::*};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};
struct Fixture {
    tmp: tempfile::TempDir,
    project: oasis7_command_center_core::domain::Project,
    db: PathBuf,
    binary: PathBuf,
    trace: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("repo");
        std::fs::create_dir(&root).unwrap();
        git(&root, &["init"]);
        git(&root, &["config", "user.name", "test"]);
        git(&root, &["config", "user.email", "test@example.org"]);
        std::fs::write(root.join("README.md"), "fixture").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-m", "fixture"]);
        let db = tmp.path().join("runtime.db");
        let mut core = CommandCenter::open(&db).unwrap();
        let project = core.open_project(&root).unwrap().project.unwrap();
        let binary = tmp.path().join("fake-codex");
        let trace = tmp.path().join("trace");
        let script=r##"#!/usr/bin/env python3
import sys,json,subprocess,time
TRACE=__TRACE__
if '--version' in sys.argv: print('codex-cli 0.137.0');sys.exit(0)
threads={};n=0;resumed=False

def send(v): print(json.dumps(v),flush=True)
def reply(m,v): send({'id':m['id'],'result':v})
def thread(t): return {'thread':{'id':t,'status':{'type':threads.get(t,'idle')}},'model':'test','cwd':__ROOT__}
def completed(t,turn,status='completed'):
 threads[t]='idle';send({'method':'turn/completed','params':{'threadId':t,'turn':{'id':turn,'status':status,'items':[],'error':None}}});send({'method':'thread/status/changed','params':{'threadId':t,'status':{'type':'idle'}}})
for line in sys.stdin:
 m=json.loads(line)
 with open(TRACE,'a') as f:f.write(json.dumps({'method':m.get('method','REPLY'),'id':m.get('id')})+'\n')
 if 'method' not in m:
  send({'method':'serverRequest/resolved','params':{'threadId':'t1','requestId':m['id']}});completed('t1','turn-t1');continue
 method=m['method'];p=m.get('params',{})
 if method=='initialize':reply(m,{'userAgent':'codex/0.137.0'})
 elif method=='thread/start':
  n+=1;t='t'+str(n);threads[t]='idle'
  if n==1:subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)'],start_new_session=True)
  reply(m,thread(t))
 elif method=='thread/resume':resumed=True;reply(m,thread(p['threadId']))
 elif method=='thread/read':reply(m,thread(p['threadId']))
 elif method=='turn/start':
  t=p['threadId'];turn='turn-'+t;mode=p['input'][0]['text'];threads[t]='active'
  send({'method':'turn/started','params':{'threadId':t,'turn':{'id':turn,'status':'inProgress','items':[],'error':None}}})
  if mode=='malformed':reply(m,{});continue
  if mode=='lost':threads[t]='idle';continue
  reply(m,{'turn':{'id':turn,'status':'inProgress','items':[],'error':None}})
  if mode=='request':
   req={'id':9007199254740993,'method':'item/tool/requestUserInput','params':{'itemId':'q1','threadId':t,'turnId':turn,'questions':[{'id':'choice','header':'Choice','question':'Choose','isSecret':True,'options':[{'label':'A','description':'A'}]}]}}
   send(req);send(req)
  elif mode=='old':send({'method':'turn/completed','params':{'threadId':t,'turn':{'id':'OLD','status':'completed','items':[],'error':None}}})
  elif mode=='background':subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)']);completed(t,turn)
  elif mode=='overflow':
   for i in range(150):send({'id':'q'+str(i),'method':'item/commandExecution/requestApproval','params':{'itemId':'i'+str(i),'startedAtMs':1,'threadId':t,'turnId':turn,'command':'echo x'}})
  elif mode=='failed':completed(t,turn,'failed')
  elif mode=='complete':
   if resumed:time.sleep(0.25)
   completed(t,turn)
 elif method=='turn/interrupt':reply(m,{});completed(p['threadId'],p['turnId'],'interrupted')
 elif method=='turn/steer':reply(m,{'turnId':p['expectedTurnId']})
"##.replace("__TRACE__",&serde_json::to_string(&trace).unwrap()).replace("__ROOT__",&serde_json::to_string(&root).unwrap());
        std::fs::write(&binary, script).unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            tmp,
            project,
            db,
            binary,
            trace,
        }
    }
    fn runtime(&self) -> Runtime {
        let mut r = Runtime::open(&self.db).unwrap();
        assert!(!r.snapshot(&self.project).unwrap().enabled);
        r.set_enabled(&self.project, true).unwrap();
        r.connect(&self.project, self.binary.to_str().unwrap())
            .unwrap();
        r
    }
    fn session(&self, r: &mut Runtime) -> String {
        let s = r
            .create_session(
                &self.project,
                SessionConfig {
                    purpose: "test work".into(),
                    scope: "fixture".into(),
                    model: None,
                    basis_revision: self.project.head.clone(),
                },
            )
            .unwrap();
        tick();
        r.snapshot(&self.project).unwrap();
        s.sessions.last().unwrap().thread_id.clone()
    }
}
fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap()
            .status
            .success()
    );
}
fn wait_idle(r: &mut Runtime) {
    let end = std::time::Instant::now() + Duration::from_secs(3);
    while !r.quit_status().safe && std::time::Instant::now() < end {
        tick();
    }
}
fn tick() {
    std::thread::sleep(Duration::from_millis(100));
}
#[test]
fn genuine_requests_preserve_big_id_target_and_receipt_with_independent_persistence() {
    let f = Fixture::new();
    let mut r = f.runtime();
    let t = f.session(&mut r);
    r.start_turn(&f.project, &t, "request").unwrap();
    tick();
    let s = r.snapshot(&f.project).unwrap();
    assert_eq!(s.requests.len(), 1);
    assert!(s.operations.iter().any(|o| o.method == "thread/start"
        && o.state == "confirmed"
        && o.thread_id.as_deref() == Some(t.as_str())));
    assert!(s.operations.iter().any(|o| o.method == "turn/start"
        && o.state == "confirmed"
        && o.thread_id.as_deref() == Some(t.as_str())
        && o.turn_id.as_deref() == Some("turn-t1")));
    let req = &s.requests[0];
    assert_eq!(req.request_id, "9007199254740993");
    let target = RuntimeTarget {
        instance_id: req.instance_id.clone(),
        thread_id: req.thread_id.clone(),
        turn_id: req.turn_id.clone(),
    };
    let answer = RequestAnswer {
        request_id: req.request_id.clone(),
        target: target.clone(),
        decision: None,
        answers: BTreeMap::from([("choice".into(), vec!["TOP-SECRET-ANSWER".into()])]),
    };
    let mut wrong = answer.clone();
    wrong.target.instance_id = "other".into();
    assert!(r.answer_request(&f.project, wrong).is_err());
    let conn = rusqlite::Connection::open(&f.db).unwrap();
    let source: String = conn
        .query_row("SELECT value FROM cc_runtime_state", [], |row| row.get(0))
        .unwrap();
    assert!(source.contains("9007199254740993"));
    // An unrelated missing receipt must not prevent management of a genuine request.
    let phantom = serde_json::json!({"id":"unknown-steer","instanceId":target.instance_id,"threadId":target.thread_id,"turnId":target.turn_id,"method":"turn/steer","state":"unknown","fingerprint":"test","createdAt":"test","error":null});
    conn.execute("INSERT INTO cc_runtime_operations(id,scope,target,state,value) VALUES('unknown-steer',?1,?2,'unknown',?3)",rusqlite::params![format!("{}:local",f.project.id),target.thread_id,phantom.to_string()]).unwrap();
    r.answer_request(&f.project, answer.clone()).unwrap();
    conn.execute(
        "UPDATE cc_runtime_operations SET state='sourceResolved' WHERE id='unknown-steer'",
        [],
    )
    .unwrap();
    tick();
    let final_s = r.snapshot(&f.project).unwrap();
    assert_eq!(final_s.requests[0].state, "resolved");
    assert!(r.answer_request(&f.project, answer).is_err());
    wait_idle(&mut r);
    assert!(
        r.quit_status().safe,
        "{:?} {:?}",
        r.quit_status(),
        r.snapshot(&f.project).unwrap()
    );
    let trace = std::fs::read_to_string(&f.trace).unwrap();
    assert!(trace.contains("9007199254740993"));
    let stored: String = conn
        .query_row(
            "SELECT group_concat(value) FROM cc_runtime_operations",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!stored.contains("TOP-SECRET-ANSWER"));
    assert!(r.shutdown_if_idle().unwrap().safe);
}
#[test]
fn lost_receipt_persists_unknown_never_replays_and_readonly_reconciliation_is_explicit() {
    let f = Fixture::new();
    let mut r = f.runtime();
    let t = f.session(&mut r);
    let s = r.start_turn(&f.project, &t, "lost").unwrap();
    let op = s
        .operations
        .iter()
        .find(|o| o.method == "turn/start")
        .unwrap();
    assert_eq!(op.state, "unknown");
    let id = op.id.clone();
    assert!(!r.quit_status().safe);
    assert!(r.start_turn(&f.project, &t, "lost").is_err());
    let reconciled = r.reconcile_operation(&f.project, &id).unwrap();
    assert!(
        reconciled
            .operations
            .iter()
            .any(|o| o.id == id && o.state == "sourceResolved")
    );
    assert!(r.quit_status().safe);
    let count = std::fs::read_to_string(&f.trace)
        .unwrap()
        .matches("turn/start")
        .count();
    assert_eq!(count, 1);
    drop(r);
    let mut reopened = Runtime::open(&f.db).unwrap();
    assert!(
        reopened
            .snapshot(&f.project)
            .unwrap()
            .operations
            .iter()
            .any(|o| o.id == id && o.state == "sourceResolved")
    );
    assert_eq!(
        std::fs::read_to_string(&f.trace)
            .unwrap()
            .matches("turn/start")
            .count(),
        1
    );
}
#[test]
fn two_owned_sessions_same_branch_old_turn_and_disable_keep_original_management() {
    let f = Fixture::new();
    let mut r = f.runtime();
    let first = f.session(&mut r);
    let second = f.session(&mut r);
    assert_ne!(first, second);
    r.start_turn(&f.project, &first, "old").unwrap();
    tick();
    let s = r.set_enabled(&f.project, false).unwrap();
    let active = s.sessions.iter().find(|s| s.thread_id == first).unwrap();
    assert_eq!(active.state, "active");
    assert!(r.start_turn(&f.project, &second, "complete").is_err());
    let target = RuntimeTarget {
        instance_id: active.instance_id.clone(),
        thread_id: first,
        turn_id: active.turn_id.clone().unwrap(),
    };
    let mut wrong = target.clone();
    wrong.turn_id = "OLD".into();
    assert!(r.interrupt_turn(&f.project, wrong).is_err());
    r.interrupt_turn(&f.project, target).unwrap();
    tick();
    assert!(r.quit_status().safe);
    assert!(r.shutdown_if_idle().unwrap().safe);
}
#[test]
fn completed_turn_with_background_process_does_not_allow_exit() {
    let f = Fixture::new();
    let mut r = f.runtime();
    let t = f.session(&mut r);
    r.start_turn(&f.project, &t, "background").unwrap();
    tick();
    let s = r.snapshot(&f.project).unwrap();
    assert_eq!(s.sessions[0].state, "completed");
    assert_eq!(s.sessions[0].background_state, "unknown");
    assert!(!r.shutdown_if_idle().unwrap().safe);
    assert!(r.start_turn(&f.project, &t, "complete").is_err());
}
#[test]
fn storage_failure_and_head_drift_prevent_wire_sideeffects() {
    let f = Fixture::new();
    let mut r = f.runtime();
    let t = f.session(&mut r);
    let conn = rusqlite::Connection::open(&f.db).unwrap();
    conn.execute_batch("CREATE TRIGGER reject_ops BEFORE INSERT ON cc_runtime_operations BEGIN SELECT RAISE(FAIL,'no storage'); END;").unwrap();
    assert!(r.start_turn(&f.project, &t, "complete").is_err());
    assert!(
        !std::fs::read_to_string(&f.trace)
            .unwrap()
            .contains("turn/start")
    );
    conn.execute_batch("DROP TRIGGER reject_ops;").unwrap();
    std::fs::write(Path::new(&f.project.path).join("new"), "new").unwrap();
    git(Path::new(&f.project.path), &["add", "."]);
    git(Path::new(&f.project.path), &["commit", "-m", "new head"]);
    assert!(r.start_turn(&f.project, &t, "complete").is_err());
    assert!(
        !std::fs::read_to_string(&f.trace)
            .unwrap()
            .contains("turn/start")
    );
    let _ = &f.tmp;
}
#[test]
fn malformed_success_and_critical_overflow_are_unknown_and_disable_new_work() {
    for mode in ["malformed", "overflow"] {
        let f = Fixture::new();
        let mut r = f.runtime();
        let t = f.session(&mut r);
        r.start_turn(&f.project, &t, mode).unwrap();
        tick();
        let s = r.snapshot(&f.project).unwrap();
        assert_eq!(s.state, "degraded");
        assert!(s.requests.len() <= 128);
        assert!(!r.quit_status().safe);
        assert!(
            r.create_session(
                &f.project,
                SessionConfig {
                    purpose: "x".into(),
                    scope: "x".into(),
                    model: None,
                    basis_revision: f.project.head.clone()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn schema_two_migration_preserves_personal_history_and_recovers_pending_operations() {
    let f = Fixture::new();
    let conn = rusqlite::Connection::open(&f.db).unwrap();
    conn.execute_batch("PRAGMA user_version=1;INSERT INTO personal(key,value) VALUES('sentinel','{}');INSERT INTO history(key,value) VALUES('sentinel','{}');").unwrap();
    let mut r = Runtime::open(&f.db).unwrap();
    assert!(!r.snapshot(&f.project).unwrap().enabled);
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |row| row.get::<_, u32>(0))
            .unwrap(),
        2
    );
    for table in ["personal", "history"] {
        assert_eq!(
            conn.query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE key='sentinel'"),
                [],
                |row| row.get::<_, u32>(0)
            )
            .unwrap(),
            1
        );
    }
}

#[test]
fn clean_history_requires_explicit_resume_and_future_database_clock_is_preserved() {
    let f = Fixture::new();
    let mut r = f.runtime();
    let t = f.session(&mut r);
    r.start_turn(&f.project, &t, "complete").unwrap();
    tick();
    r.snapshot(&f.project).unwrap();
    wait_idle(&mut r);
    assert!(r.shutdown_if_idle().unwrap().safe);
    drop(r);
    let conn = rusqlite::Connection::open(&f.db).unwrap();
    conn.execute(
        "UPDATE cc_runtime_state SET sequence=sequence+100000000000000",
        [],
    )
    .unwrap();
    let mut r = Runtime::open(&f.db).unwrap();
    assert!(r.snapshot(&f.project).unwrap().sessions[0].restorable_history);
    r.connect(&f.project, f.binary.to_str().unwrap()).unwrap();
    assert!(r.start_turn(&f.project, &t, "complete").is_err());
    r.continue_session(&f.project, &t).unwrap();
    r.start_turn(&f.project, &t, "complete").unwrap();
    // A receipt confirms turn creation, not asynchronous completion or background-idle proof.
    wait_idle(&mut r);
    let status = r.shutdown_if_idle().unwrap();
    assert!(
        status.safe,
        "resumed turn never reached verified idle: {status:?}"
    );
}
