#!/usr/bin/env python3
import importlib.util,json,pathlib,tempfile,types
p=pathlib.Path(__file__).with_name("github-project-task.py")
s=importlib.util.spec_from_file_location("task",p); m=importlib.util.module_from_spec(s); s.loader.exec_module(m)
uid="task_11111111111111111111111111111111"
live={"task_uid":uid,"title":"t","issue_number":1,"issue_url":"https://github.com/o/r/issues/1","status":"committed","acceptance":[]}
m.github_issue_record=lambda *_:dict(live)
m.authoritative_repository_identity=lambda *_:{"repository":"o/r","default_branch":"main","canonical_worktree":"/tmp/w","task_branch":"task/x"}
for existing in (True,False):
 with tempfile.TemporaryDirectory() as td:
  root=pathlib.Path(td); (root/".pm/github-project-sync").mkdir(parents=True)
  record=dict(live); record["project_item_id"]="ITEM" if existing else ""
  (root/".pm/github-project-sync/tasks.json").write_text(json.dumps({"version":1,"tasks":{uid:record}}))
  calls=[]
  def gql(_query,_variables):
   calls.append(1)
   node={"id":"ITEM","project":{"number":1},"fieldValues":{"nodes":[]}}
   return {"data":{"nodes":[node]}} if existing else {"data":{"search":{"nodes":[{**live,"body":f"task_uid: {uid}","number":1,"url":live["issue_url"],"projectItems":{"nodes":[node]}}]}}}
  m.project_refresh_graphql=gql
  args=types.SimpleNamespace(root=root,mapping=".pm/github-project-sync/tasks.json",task_uid=uid,repo="o/r",project_owner="o",project_number=1,json=True)
  m.command_refresh_task(args)
  assert len(calls)==1,(existing,calls)
print("refresh-graphql-call-budget.test: OK")
