#!/usr/bin/env bash
# Test-only transport for deterministic resource-cleanup process-use coverage.
# Source this file and install the shims into the disposable fixture's PATH.
oasis7_install_complete_process_probe() {
  local bin_dir="$1"
  mkdir -p "$bin_dir"
  cat >"$bin_dir/ps" <<'PY'
#!/usr/bin/env python3
import os,sys
if sys.argv[1:] != ["-axo", "pid=,ppid=,uid=,command="]:
    raise SystemExit("unexpected fixture ps invocation: " + repr(sys.argv[1:]))
try:
    uid=os.getuid()+1
except AttributeError:
    uid=1001
print(f"424242 1 {uid} fixture-process-with-path-free-argv")
PY
  cat >"$bin_dir/lsof" <<'PY'
#!/usr/bin/env python3
import sys
args=sys.argv[1:]
if args[:3] != ["-n", "-F", "pfnt"] or "-p" not in args:
    raise SystemExit("unexpected fixture lsof invocation: " + repr(args))
for pid in args[args.index("-p")+1].split(","):
    print("p"+pid)
    print("fcwd")
    print("tDIR")
    print("n/tmp")
    print("f0")
    print("tCHR")
    print("n/dev/null")
PY
  chmod +x "$bin_dir/ps" "$bin_dir/lsof"
}
