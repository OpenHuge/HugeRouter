# OpenHuge 后端交付物存储 + 兑换激活事实验收记录

日期：2026-05-06
分支：lab

## 范围

- 完成交付物存储强前置：`delivery_artifacts` 表、元数据 API、DB inline 密文存储、16 MiB payload 上限、替换 supersede 语义。
- 完成兑换与激活事实：`delivery_activations` 表、`POST /v1/delivery-activations/redeem`、`GET /v1/delivery-activations/{activation_id}`。
- 激活采用明确拒绝重复兑换策略：首次成功后 redemption code 状态变为 `used`，后续返回 `redemption_code_used`。
- 激活事实绑定兑换时的 `delivery_id`、内部 `code_id`、`artifact_id`、`entitlement_id`、tenant/project/provider、artifact metadata summary、entitlement ends_at。

## 安全与边界

- redemption_code 明文只在请求 handler 内短暂出现，Store 只接收 hash。
- 激活响应不包含 redemption_code、browser_file_unlock_code、download_token、download_url、payload_base64、ciphertext 或 `.hcbrowser` 内容。
- artifact 查询只返回 metadata，不返回密文 payload。
- 无 active artifact、过期 code、撤销 code、重复 code、跨租户未授权请求均阻断。
- 本轮未实现下载授权、文件下载、前端兑换输入、客户端恢复、TS client 消费、前后端联调、部署。

## 关键实现

- `services/control-plane-api/src/store_schema.rs` 和 `infra/sql/runtime-schema.sql` 新增 `delivery_artifacts`、`delivery_activations`。
- `services/control-plane-api/src/store.rs` 新增 artifact 存储和 activation 原子兑换逻辑；Postgres 路径在事务中锁定 redemption code、delivery、entitlement、active artifact，并原子插入 activation 与更新 code used。
- `services/control-plane-api/src/lib.rs` 新增 artifact metadata API 与 activation redeem/get API。
- `crates/protocol-ir/src/lib.rs`、`openapi_docs.rs`、`artifacts.rs`、`examples.rs` 新增协议结构、OpenAPI、schema、example、operation metadata 登记。

## 验证命令

- `cargo fmt --all`：通过。
- `pnpm.CMD generate`：通过；存在环境警告 `node 24.15.0 wanted, current v24.12.0`，未阻断生成。
- `cargo check -p control-plane-api -p protocol-ir --locked`：通过。
- `cargo test -p control-plane-api --locked`：76 passed。
- `cargo test -p protocol-ir --locked`：8 passed。
- `cargo clippy -p control-plane-api -p protocol-ir --all-targets --locked -- -D warnings`：通过。

## 验收结论

后端已能在存在 prepared delivery、active redemption_code、active entitlement、active artifact 的前提下，将 redemption_code 兑换为 activation 主事实，标记 code used，并返回不含下载授权和文件内容的激活投影。当前任务计划书的后端边界已闭合。
