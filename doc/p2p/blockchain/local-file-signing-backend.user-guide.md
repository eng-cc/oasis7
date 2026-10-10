# 本地签名工具使用手册

本手册面向已安装工具的管理员和调用者。安装、版本绑定及部分失败恢复见 [operator runbook](local-file-signing-backend.runbook.md)，安全设计见 [design](local-file-signing-backend.design.md)。业务方决定何时签名和如何使用结果；工具负责校验授权、保管私钥和记录请求。

## 支持范围

当前 M0 只支持 `rollback_strict_audit_v1`，使用 Ed25519 签署经过协议校验的 canonical rollback payload。不能直接签任意文件、交易或发布产物，也不能仅通过修改 purpose 扩展用途。新用途需要实现对应协议适配。

安装成功不等于签名已启用。无密钥安装可以完成权限边界验收，但缺少 policy 时 `doctor` 返回 `AUTHORIZATION_DENIED`、`ready: false` 是预期状态。管理员后续按需创建密钥、安装 policy 和 grant、启用 purpose；本手册中的命令不会自动执行。

## 角色和入口

| 角色 | 入口 | 职责 |
| --- | --- | --- |
| 已绑定调用者 | `oasis7_local_signer` | doctor、准备 job、提交、查询 |
| 主机管理员 | `oasis7_local_signer_admin`，以 root 执行 | 创建密钥、安装授权、启停、撤销 |
| 隔离 signer | `oasis7_local_signer_worker` | 由调用者 CLI 通过固定 sudo 规则调用 |

从 `/private/etc/oasis7/local-signer-installation.json` 确认当前 release、caller 和 work_dir。使用已安装 release 的绝对路径，不使用工作区编译产物替换管理员入口。下文 `$RELEASE` 是经核对的 `/usr/local/libexec/oasis7-local-signer/<release_id>`；`$JOB` 是本次唯一 job ID。普通 CLI 必须以绑定 caller 的真实身份运行，不能以 root 代替。不要直接调用 worker 或给 caller 增加 admin/shell 权限。

## 管理员：首次启用

先以实际 caller 执行 `$RELEASE/oasis7_local_signer doctor`，核对安装状态。随后由管理员在自己的终端执行以下各步；每个候选的内容和 SHA-256 应先核对，不能从示例复制占位值后直接应用。

```sh
sudo "$RELEASE/oasis7_local_signer_admin" key-create --signer-id "$SIGNER_ID" --purpose rollback_strict_audit_v1
sudo "$RELEASE/oasis7_local_signer_admin" key-public --signer-id "$SIGNER_ID" --format hex
sudo "$RELEASE/oasis7_local_signer_admin" policy-install --candidate "$POLICY_JSON" --expected-sha256 "$POLICY_SHA256"
sudo "$RELEASE/oasis7_local_signer_admin" approve-batch --candidate "$GRANT_JSON" --expected-sha256 "$GRANT_SHA256"
sudo "$RELEASE/oasis7_local_signer_admin" enable --purpose rollback_strict_audit_v1
```

`key-create` 输出公钥，不导出私钥；已有身份不能当作新密钥重复创建。公钥指纹是原始 32 字节公钥的 SHA-256，不是 hex 文本的 SHA-256。候选文件必须位于已绑定 caller 的 work root 下的合法 job ID 子目录中（如 work_dir/admin-job/policy.json），归对应 caller 所有，为无符号链接、单硬链接、group/other 不可写的普通文件，大小不超过 1 MiB。管理员命令检查固定安装后再读取候选。

### Policy 和 grant 的内容

精确字段与校验规则见 [types.rs](../../../crates/oasis7_local_signer/src/types.rs)。JSON 不接受未知字段；所有 SHA-256 使用 64 位小写 hex。

| 候选 | 必须绑定的内容 |
| --- | --- |
| Policy `oasis7.local_signer_policy.v2` | installation_id、deployment_id、policy_revision、key_bindings、limits；安装时 enabled_purposes 必须为空 |
| key binding | purpose、signer_id、公钥指纹及允许的 protocol_authorities、provider_ids、deployment_ids、network_ids |
| Grant `oasis7.local_signing_batch_grant.v1` | grant_id、installation/deployment、caller_uid、network_id、task_uid、source_head_oid、policy_revision、有效时间、approval_record_ref、items |
| grant item | purpose、provider_id（当前为显式 null）、signer_id、公钥指纹、operation_key、payload_sha256、protocol_bindings、max_distinct_requests |

`not_before` 和 `expires_at` 是 UTC Unix 毫秒，前者必须小于后者。source_head_oid 是 40 位小写 Git OID。policy 上限为 64 个 batch item、每项 16 个 distinct request、payload 16 MiB；实际使用可设更小值。

`operation_key` 必须用 [identity::operation_key](../../../crates/oasis7_local_signer/src/identity.rs) 的算法生成：SHA-256(domain `oasis7.local-operation.v2\0` + installation/deployment/network/purpose/signer 的 UTF-8 字节，分别加 4 字节大端长度前缀 + 原始 32 字节 payload SHA-256)。不能用 job ID 或自行拼接字符串替代。协议绑定包含 authority_id、rollback_ticket、receipt_id、nonce；它们必须与 payload 和请求一致。

启用后以实际 caller 再运行 `doctor`。`ready: true` 只表示 policy 的 purpose 已启用，不保证某个请求拥有有效 grant、密钥或剩余额度；提交时仍逐项校验。

## 调用者：一次签名

1. 执行 `"$RELEASE/oasis7_local_signer" prepare --job-id "$JOB"`。空 job 返回 `JOB_DIRECTORY_READY` 和真实 work_dir。
2. 在该目录放入 `input.json` 和 `payload.bin`，使用 caller 所有的普通文件和私有目录，不使用符号链接。payload 必须来自业务协议生成器；[rollback.rs](../../../crates/oasis7_local_signer/src/rollback.rs) 校验其 canonical 编码和协议绑定。
3. 再执行同一 `prepare`，预期 `PREPARED`。它生成 `request.json` 与 `request.payload.sha256` 并固定 request_id；保存这些文件。
4. 执行 `"$RELEASE/oasis7_local_signer" submit --job-id "$JOB"`。检查退出码和完整 JSON；成功必须是 `status: committed` 且 error_code 为 null。
5. 保存公开结果，核对 request_id、operation_key、公钥、signature_base64 和 audit_digest；业务消费者使用协议规定的消息及公钥验证签名，不以 CLI 成功代替业务验签。

`input.json` 结构如下，占位值由业务方填入；null 字段不能省略：

```json
{
  "schema_version": "oasis7.local_signer_job_input.v1",
  "installation_id": "INSTALLATION_ID",
  "purpose": "rollback_strict_audit_v1",
  "provider_id": null,
  "signer_id": "SIGNER_ID",
  "grant_id": "GRANT_ID",
  "context": {
    "deployment_id": "DEPLOYMENT_ID",
    "network_id": "NETWORK_ID",
    "task_uid": "TASK_UID",
    "source_head_oid": "REPLACE_WITH_40_LOWERCASE_HEX",
    "protocol_context": {
      "authority_id": "AUTHORITY_ID",
      "rollback_ticket": "ROLLBACK_TICKET",
      "receipt_id": "RECEIPT_ID",
      "nonce": "NONCE"
    }
  }
}
```

ID 一般为 2–128 字节 ASCII，首字符为字母或数字，其余允许字母、数字、点、下划线、连字符。不要修改已准备 job 的 input、payload 或生成元数据；内容变化会触发 `ID_CONFLICT`。新业务请求使用新 job，但不能通过新 job 绕过 grant 的请求额度。

## 查询、停用和撤销

提交超时或结果不明时，保留原 job，先执行 `"$RELEASE/oasis7_local_signer" inspect --job-id "$JOB"` 查询已有 request。不要删除元数据或立即创建新请求重试。恢复前核对服务端状态、同一请求身份及当前授权；grant 过期或撤销后也不能假设旧请求必然可重放。

```sh
sudo "$RELEASE/oasis7_local_signer_admin" disable --purpose rollback_strict_audit_v1
sudo "$RELEASE/oasis7_local_signer_admin" revoke-grant --grant-id "$GRANT_ID"
```

disable 关闭 purpose；revoke-grant 保留授权记录并阻止后续使用。已发出的签名不会因此消失，业务侧仍需处理自己的撤销语义。

## 错误处理与能力边界

| 错误 | 下一步 |
| --- | --- |
| INVALID_INPUT / ID_CONFLICT | 核对 schema、显式 null、canonical payload 及原 job；保留原请求 |
| AUTHORIZATION_DENIED / KEY_OR_AUTHORITY_UNAVAILABLE / BUDGET_EXHAUSTED | 核对 purpose、key binding、grant、时间和额度；由管理员处理 |
| LOCK_BUSY | 保留 job，等待正在执行的操作结束 |
| PERSISTENCE_FAILED / RECOVERY_REQUIRED / INSTALLATION_DRIFT | 停止依赖动作，保留安装、journal、receipt 和 job，按 operator runbook 诊断 |
| WORKER_TIMEOUT / IPC_PROTOCOL_ERROR / WORKER_FAILED | 结果可能不明；先 inspect，不盲目重试 |
| UNSUPPORTED_PLATFORM_OR_FS | 核对受支持平台、文件系统和受保护路径，不放宽权限绕过 |

备份及恢复命令目前未实现，会拒绝操作；不能将其输出当作备份成功。主机安装走已批准 packaged installer，admin 的 install 命令不执行安装。私钥不导出，不在 caller jobs、日志或仓库保存私有 custody/control 数据。部署状态、业务批准记录和签名结果应分别保留；不要修改历史 journal 来制造成功状态。
