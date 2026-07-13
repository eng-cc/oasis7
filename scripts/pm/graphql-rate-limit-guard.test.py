#!/usr/bin/env python3
import importlib.util, pathlib

path = pathlib.Path(__file__).with_name("github-project-sync.py")
spec = importlib.util.spec_from_file_location("sync", path)
sync = importlib.util.module_from_spec(spec); spec.loader.exec_module(sync)

def outcome(value):
    sync.run_json = lambda *_a, **_k: {"data":value}
    return sync.broad_rate_limit_guard("token")

assert outcome({"rateLimit":{"remaining":99,"resetAt":"2099-01-01T00:00:00Z"}})["reason"] == "graphql_budget_insufficient"
assert outcome({"rateLimit":{"remaining":None,"resetAt":""}})["reason"] == "graphql_rate_limit_unknown"
sync.run_json = lambda *_a, **_k: (_ for _ in ()).throw(RuntimeError("offline"))
assert sync.broad_rate_limit_guard("token")["reason"] == "graphql_rate_limit_unavailable"
assert outcome({"rateLimit":{"remaining":100,"resetAt":"2099-01-01T00:00:00Z"}})["status"] == "ok"
print("graphql-rate-limit-guard.test: OK")
