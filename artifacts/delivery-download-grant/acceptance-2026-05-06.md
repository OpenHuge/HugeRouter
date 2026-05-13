# OpenHuge 后端下载授权与取回验收证据

日期：2026-05-06
分支：lab
任务计划书：OpenHuge-后端下载授权与取回-任务计划书-2026-05-06.md

## 交付范围

- 新增下载授权事实 `delivery_download_grants`，绑定 `activation_id + artifact_id + entitlement_id`。
- 下载授权从有效 `activation_id` 签发，不接受 `redemption_code` 或直接 `artifact_id`。
- 签发响应一次性返回 `download_token` 明文；存储层仅保存 token hash、prefix、last4、状态、过期时间和使用次数。
- 首版 token TTL 为 15 分钟，`max_uses = 1`。
- 取回接口仅接受 `Authorization: Bearer <download_token>`，路由为 `GET /v1/delivery-downloads/artifact`，不通过 path/query 传 token。
- 取回成功只返回加密 `.hcbrowser` ciphertext，不返回解密信息、`browser_file_unlock_code`、cookie 或 auth 明文。
- 成功取回会原子标记 token used；重复/并发使用只有一次成功。
- 取回时重新校验 activation、entitlement、artifact 状态，权益撤销后不能继续下载。

## 验证命令

```powershell
$tool="D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin"
$env:PATH="$tool;$env:PATH"
$env:CARGO_HOME="D:/OpenHuge/.toolchains/cargo"
$env:RUSTUP_HOME="D:/OpenHuge/.toolchains/rustup"
cargo fmt --all
cargo check -p control-plane-api -p protocol-ir --locked
cargo test -p control-plane-api delivery_download_grant --locked
pnpm.CMD generate
cargo test -p control-plane-api -p protocol-ir --locked
cargo clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings
```

## 验证结果

- `cargo check -p control-plane-api -p protocol-ir --locked`：通过。
- `cargo test -p control-plane-api delivery_download_grant --locked`：3 个新增下载授权测试通过。
- `pnpm.CMD generate`：通过；存在 Node engine 警告，期望 `24.15.0`，当前 `v24.12.0`。
- `cargo test -p control-plane-api -p protocol-ir --locked`：通过，control-plane-api 79 个测试通过，protocol-ir 8 个测试通过。
- `cargo clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings`：通过。

## 关键测试覆盖

- `delivery_download_grant_issues_once_and_retrieves_bound_ciphertext`
  - 验证 token 明文仅签发响应返回。
  - 验证查询 grant 不返回 `download_token` / `token_hash`。
  - 验证取回返回原始加密 ciphertext 和 artifact header。
  - 验证第二次使用同 token 返回 `download_token_used`。

- `delivery_download_grant_rejects_missing_invalid_expired_revoked_and_concurrent_tokens`
  - 验证缺失 header、无效 token、过期 token、撤销 token 均被拒绝。
  - 验证并发取回同一 token 只有一次成功。

- `delivery_download_grant_rechecks_entitlement_and_tenant_boundaries`
  - 验证跨租户查询、撤销、签发被拒绝。
  - 验证签发后撤销 delivery/entitlement 会阻断取回，且失败取回不会消耗 token。
