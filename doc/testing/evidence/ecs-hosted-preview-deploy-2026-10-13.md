# ECS Hosted Preview Deployment - 2026-10-13

- Owner role: `blockchain_ops_engineer`
- Scope: reset ECS host, isolated DevLocal node and `hosted_public_join` player surface; no formal world activation or public-testnet fleet rollout.
- Source: `f69c8a666024fa564e52314965025b019fa42d4c`.
- Package workflow: [Testnet Packages run 38420634060](https://github.com/eng-cc/oasis7/actions/runs/38420634060), `linux_only`, `packaging`.
- Current observation: deployment preparation only; the first native package attempt failed and the same-source native retry is in progress; its result is pending. Runtime installation and invitation acceptance have not passed.

## Deployment preparation observed

The reset host is Ubuntu 24.04 x86_64, with two CPUs, approximately 3.5 GB RAM and 26 GB available disk. Fresh inspection found no Oasis7 services or world data. The confirmed replacement SSH host key is pinned for this operation without disabling SSH host verification.

`bootstrap1.oasis7.tech` resolves to this ECS. HTTPS certificate issuance and server-side `nginx -t` passed; an external Python HTTPS client verified the certificate and Chrome reached the maintenance response. The public endpoint currently returns 503. The local LibreSSL curl client reset during the TLS handshake; that result is retained separately from successful clients.

The prepared services use a dedicated unprivileged account, immutable release paths, loopback-only runtime/control bindings and private persistent state. The player environment uses explicit file account/session/replay stores and a newly generated registration issuer. Test login remains disabled, no synthetic strong-auth material is provisioned, and no test email or model call was sent. SMTP TLS and account authentication succeeded without sending a message; OTP delivery and full player login remain untested. Secret values are not recorded here.

Independent static review covered nginx, service units and a redacted environment summary. It required rejecting foreign Origin/Host and removing query-bearing logs from every vhost; those changes were made. Runtime negative checks and restart verification remain outstanding.

## Package failure retained

Attempt 1 completed Web delivery successfully. The native `Build launcher bundle` process terminated at `2026-10-13T15:52:14Z` with exit code 143 (`SIGTERM`), after the core library compilation warnings. No Rust compiler error was reported. Both post-job cache/checkout actions were skipped. GitHub annotations only repeat the process exit and a Node.js deprecation warning; they do not establish the termination cause. Attempt 2 is running under the [same run, attempt 2](https://github.com/eng-cc/oasis7/actions/runs/38420634060/attempts/2); observed at `2026-10-13T16:00:58.376549+00:00`. The failure is retained; no native package was uploaded or installed, and checks are not bypassed.

## Acceptance boundary and remaining work

This fresh node is planned as a separate DevLocal standalone prototype with world identity `oasis7-ecs-preview-20261013`; it does not resume the former public-testnet node or grant it validator membership. A single ECS and local state directories are one failure domain.

Formal activation, ordinary proof-consumer integration, independent-failure-domain durable confirmation/recovery and old-writer fencing remain the current [infrastructure implementation gaps](../../engineering/command-center/README.md#1-持久单权威与同世界恢复). Public Game HTTP writes, a valid external executor/provider and same-candidate autonomous production/revisit evidence are not established by this deployment preparation.

The [current stage requirements](../../core/prd.md#当前阶段交付目标与全局-p0), [formal authority contract](../../p2p/design.md#des-p2p-formal-controlled-authority-v1) and [hosted operator procedure](../../p2p/blockchain/hosted-player-access-operator-runbook.md) still govern invitation acceptance. A successful package build, HTTPS page or local restart cannot substitute for them. No invite-test-ready or formal persistent-world claim is made.
