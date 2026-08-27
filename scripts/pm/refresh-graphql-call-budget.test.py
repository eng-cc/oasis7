#!/usr/bin/env python3
import importlib.util,json,pathlib,tempfile,types
p=pathlib.Path(__file__).with_name("github-project-task.py")
s=importlib.util.spec_from_file_location("task",p); m=importlib.util.module_from_spec(s); s.loader.exec_module(m)
uid="task_11111111111111111111111111111111"
live={"task_uid":uid,"title":"t","issue_number":1,"issue_url":"https://github.com/o/r/issues/1","status":"committed","acceptance":[],"owner_role":"tpm","module":"engineering","priority":"P2","worktree_hint":"/tmp"}
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
   node={"id":"ITEM","project":{"id":"PROJECT_ID","number":1,"owner":{"login":"o"}},"fieldValues":{"pageInfo":{"hasNextPage":False},"nodes":[]}}
   return {"data":{"nodes":[node]}} if existing else {"data":{"search":{"nodes":[{**live,"body":f"task_uid: {uid}","number":1,"url":live["issue_url"],"projectItems":{"nodes":[node]}}]}}}
  m.project_refresh_graphql=gql
  args=types.SimpleNamespace(root=root,mapping=".pm/github-project-sync/tasks.json",task_uid=uid,repo="o/r",project_owner="o",project_number=1,json=True)
  m.command_refresh_task(args)
  assert len(calls)==1,(existing,calls)


def refresh_node(owner="o", project_id="PROJECT_ID", page_info="complete"):
    field_values={"nodes":[
        {"name":"In Progress","field":{"name":"Status"}},
        {"text":uid,"field":{"name":"Task UID"}},
        {"name":"tpm","field":{"name":"Owner Role"}},
        {"name":"engineering","field":{"name":"Module"}},
        {"name":"committed","field":{"name":"PM Status"}},
        {"name":"execution","field":{"name":"Workflow Phase"}},
        {"name":"P2","field":{"name":"Priority"}},
        {"text":"/tmp/w","field":{"name":"Canonical Worktree"}},
        {"name":"n/a","field":{"name":"Test Tier Required"}},
    ]}
    if page_info == "complete":
        field_values["pageInfo"]={"hasNextPage":False}
    elif page_info == "null":
        field_values["pageInfo"]=None
    elif page_info == "invalid":
        field_values["pageInfo"]={"hasNextPage":"false"}
    elif page_info == "true":
        field_values["pageInfo"]={"hasNextPage":True}
    return {
        "id":"ITEM",
        "project":{"id":project_id,"number":1,"owner":{"login":owner}},
        "fieldValues":field_values,
    }


def invoke_refresh(existing, node, canonical_project=None, expect_success=False, include_project=True):
    with tempfile.TemporaryDirectory() as td:
        root=pathlib.Path(td); (root/".pm/github-project-sync").mkdir(parents=True)
        record=dict(live); record["project_item_id"]="ITEM" if existing else ""
        project=canonical_project or {"owner":"o","number":1}
        mapping={"version":1,"tasks":{uid:record}}
        if include_project:
            mapping["project"]=project
        mapping_path=root/".pm/github-project-sync/tasks.json"
        mapping_path.write_text(json.dumps(mapping)+"\n")
        before=mapping_path.read_text()
        calls=[]
        def gql(query, _variables):
            calls.append(query)
            if existing:
                return {"data":{"nodes":[node]}}
            issue={**live,"body":f"task_uid: {uid}","number":1,"url":live["issue_url"],
                   "projectItems":{"nodes":[node]}}
            return {"data":{"search":{"nodes":[issue]}}}
        m.project_refresh_graphql=gql
        args=types.SimpleNamespace(root=root,mapping=".pm/github-project-sync/tasks.json",task_uid=uid,
                                   repo="o/r",project_owner="o",project_number=1,json=True)
        if expect_success:
            m.command_refresh_task(args)
            assert len(calls)==1,(existing,calls)
            assert "owner" in calls[0] and "pageInfo" in calls[0], calls[0]
        else:
            try:
                m.command_refresh_task(args)
            except SystemExit as exc:
                assert exc.code == 1, (existing, exc.code)
            else:
                raise AssertionError((existing, "refresh unexpectedly succeeded"))
            assert mapping_path.read_text() == before, (existing, mapping_path.read_text())


for existing in (True,False):
    invoke_refresh(existing, refresh_node(), expect_success=True)
    invoke_refresh(existing, refresh_node(owner="foreign-owner"))
    for page_info in ("missing", "null", "invalid", "true"):
        node=refresh_node(page_info=page_info)
        invoke_refresh(existing, node)

invoke_refresh(
    True,
    refresh_node(project_id="FOREIGN_PROJECT_ID"),
    canonical_project={"id":"CANONICAL_PROJECT_ID","owner":"o","number":1},
)


def test_refresh_restores_project_and_selected_audit():
    workflow_path=pathlib.Path(__file__).with_name("github-project-workflow.py")
    workflow_spec=importlib.util.spec_from_file_location("workflow",workflow_path)
    workflow=importlib.util.module_from_spec(workflow_spec); workflow_spec.loader.exec_module(workflow)
    with tempfile.TemporaryDirectory() as td:
        root=pathlib.Path(td); (root/".pm/github-project-sync").mkdir(parents=True)
        record=dict(live); record["project_item_id"]="ITEM"
        mapping_path=root/".pm/github-project-sync/tasks.json"
        mapping_path.write_text(json.dumps({"version":1,"tasks":{uid:record}})+"\n")
        node=refresh_node()
        calls=[]
        def gql(query, _variables):
            calls.append(query)
            return {"data":{"nodes":[node]}}
        m.project_refresh_graphql=gql
        args=types.SimpleNamespace(root=root,mapping=".pm/github-project-sync/tasks.json",task_uid=uid,
                                   repo="o/r",project_owner="o",project_number=1,json=True)
        m.command_refresh_task(args)
        refreshed=json.loads(mapping_path.read_text())
        assert refreshed["project"] == {"id":"PROJECT_ID","number":1,"owner":"o","repo":"o/r"}, refreshed
        assert len(calls)==1
        audit_node=dict(node, content={"body":f"task_uid: {uid}","number":1,"title":"t","url":live["issue_url"]})
        workflow.run_json=lambda _cmd: {"data":{"nodes":[audit_node]}}
        audit_args=types.SimpleNamespace(root=root,mapping=".pm/github-project-sync/tasks.json",task_uid=uid,
                                         repo="o/r",project_owner="o",project_number=1,status=None,
                                         include_done=False,full_list=False,global_maintenance=False,
                                         strict_mapping=False,limit=1000,json=True)
        assert workflow.command_audit(audit_args)==0


test_refresh_restores_project_and_selected_audit()
print("refresh-graphql-call-budget.test: OK")
