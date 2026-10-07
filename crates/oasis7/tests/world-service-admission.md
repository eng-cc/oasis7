# HTTP admission bounds

The chain runtime HTTP listener uses conservative safety defaults: 32 active
connections globally, four per source IP, and a 32-request burst per IP refilled
at 32 requests per second. Admission state holds at most 1024 IP entries and
expires inactive entries after 60 seconds. A permit is released when its handler
exits, including error paths. Global saturation returns 503; per-IP or admission
state saturation returns 429. Both advertise `Retry-After: 1` seconds.

Requests retain the existing 64 KiB total HTTP byte limit (including headers),
with a two-second total read deadline and a two-second response write deadline.
Oversize requests return 413. Slow incomplete requests return 408. Rejection
responses themselves use bounded writes; overload writes allow at most 10 ms
on the accept loop. These bounds apply to existing routes as well as the world
service, without changing signed request bytes or World authorization.

These are a bounded local operating point, not a topology or fairness guarantee.
Sessions sharing an IP share admission limits: four slow connections can delay
another session until the deadline releases capacity and its cooldown expires.
Other IPs retain independent per-peer capacity subject to the global ceiling.
Clients must honor bounded cooldown, retain original signed Submit identity,
and use Lookup after unknown outcomes rather than regenerate or replay Submit.

Conformance must exercise slow streams, oversized bodies, rate bursts and
healthy-session progress (including recovery for sessions sharing an IP), not
only the pure admission helper tests. Production-scale quotas and deployment
capacity have not been evaluated by this prerequisite fixture.
