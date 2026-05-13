# OpenHuge 第一阶段后端交付事实源验收记录

## 范围

- 已执行计划书：`OpenHuge-第一阶段后端交付事实源-任务计划书-2026-05-05.md`
- 执行分支：`lab`
- 执行日期：2026-05-06

## 已交付

- 新增后端事实：`deliveries`、`delivery_codes`、`delivery_entitlements`
- 新增管理接口：
  - `POST /v1/deliveries/prepare`
  - `GET /v1/deliveries/{delivery_id}`
  - `POST /v1/deliveries/{delivery_id}/revoke`
- 交付码格式：
  - `ku0-red-v1-YYMMDD-XXXX-RRRRRRRRRRRR-CC`
  - `ku0-brw-v1-YYMMDD-XXXX-RRRRRRRRRRRR-CC`
- 明文码只在 prepare 响应一次性返回；存储层只保留 hash、prefix、last_four、状态和审计字段。
- revoke 会联动 delivery、code、entitlement 状态失效。
- OpenAPI、JSON Schema、example、TS operation metadata 已生成。

## 明确未进入本阶段

- 未实现 `.hcbrowser` 上传、下载、存储或解密。
- 未做前端联调或 UI 改造。
- 未做微信登录/支付改造。
- 未做服务器部署。
- 未复用或改造 `opening_grants` 语义。

## 验证命令

```powershell
$env:PATH="D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin;$env:PATH"
& "D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin/cargo.exe" fmt --all
& "D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin/cargo.exe" check -p control-plane-api -p protocol-ir --locked
& "D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin/cargo.exe" test -p control-plane-api --locked
& "D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin/cargo.exe" test -p protocol-ir --locked
& "D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin/cargo.exe" clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings
$env:HUGEROUTER_RUST_BIN="D:/OpenHuge/.toolchains/rustup/toolchains/1.94.1-x86_64-pc-windows-msvc/bin"
pnpm.CMD generate
```

## 验证结果

- `cargo check -p control-plane-api -p protocol-ir --locked`：通过
- `cargo test -p control-plane-api --locked`：69 passed
- `cargo test -p protocol-ir --locked`：8 passed
- `cargo clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings`：通过
- `pnpm.CMD generate`：通过；环境提示 Node 期望 `24.15.0`，当前 `24.12.0`，未阻塞生成。

## 测试覆盖

- 创建交付时同步创建 delivery/code/entitlement 三类事实。
- get 投影不返回明文码，也不暴露 code_hash。
- delivery code hash 不等于明文，且存储为 SHA-256 hex。
- redemption code 与 browser file unlock code 均校验格式、长度、大小写和 checksum。
- revoke 后 delivery/code/entitlement 均失效。
- 非本租户或无管理权限用户不能 prepare/get/revoke。
