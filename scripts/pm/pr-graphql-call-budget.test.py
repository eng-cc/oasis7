#!/usr/bin/env python3
import importlib.util,json,pathlib
p=pathlib.Path(__file__).with_name("pr-lifecycle-gate.py")
s=importlib.util.spec_from_file_location("gate",p); gate=importlib.util.module_from_spec(s); s.loader.exec_module(gate)
calls=[]
def response(overflow=False):
 return {"data":{"repository":{"pullRequest":{"comments":{"pageInfo":{"hasNextPage":overflow},"nodes":[]},"reviews":{"pageInfo":{"hasNextPage":False},"nodes":[]},"reviewThreads":{"pageInfo":{"hasNextPage":False},"nodes":[]},"commits":{"nodes":[{"commit":{"statusCheckRollup":{"contexts":{"pageInfo":{"hasNextPage":False},"nodes":[]}}}}]}}}}}
gate._run_json=lambda cmd:(calls.append(cmd) or response(False))
snapshot=gate.graphql_pr_snapshot("eng-cc/oasis7",1)
assert len(calls)==1 and set(snapshot)=={"comments","reviews","threads","statusCheckRollup"}
query=next(arg.removeprefix("query=") for arg in calls[0] if arg.startswith("query="))
assert query.count("{")==query.count("}"), query
calls.clear(); gate._run_json=lambda cmd:(calls.append(cmd) or response(True))
try: gate.graphql_pr_snapshot("eng-cc/oasis7",1)
except SystemExit as exc: assert "exceeded 100 nodes" in str(exc)
else: raise AssertionError("overflow did not fail closed")
assert len(calls)==1
print("pr-graphql-call-budget.test: OK")
