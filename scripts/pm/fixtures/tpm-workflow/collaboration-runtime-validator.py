#!/usr/bin/env python3
import json,sys
v=json.load(sys.stdin)
print(json.dumps({"status":"live_ack","dispatch_ack":v["dispatch_ack"],"agent_id":v["agent_id"],"artifact_digest":v["artifact_digest"]}))
