# 服务器部署与更新 Runbook

[返回文档索引](../README.md)

最后更新：2026-05-07

这份文档用于记录当前 OpenHuge 自托管服务器的实际部署形态，以及后续热更新、全量更新、验证、回滚的标准操作。本文档故意不记录面板密码、SSH 私钥、API token、本地客户端 token、兑换码、数据库密码等敏感信息。

## 1. 当前服务器情况

当前已确认的业务入口：

- 公开 API 基础地址：`http://175.27.228.168`
- 公开健康检查：`http://175.27.228.168/healthz`
- 管理面板端口：`37860`，只作为管理入口，不是业务 API 入口

服务器上的应用目录：

- 应用根目录：`/opt/openhuge`
- 当前生效源码目录：`/opt/openhuge/source`
- Compose 文件：`/opt/openhuge/compose.yaml`
- 环境变量文件：`/opt/openhuge/.env`
- Postgres Docker volume：`openhuge_postgres_data`
- 2026-05-07 更新时确认过的源码备份：`/opt/openhuge/source-backup-20260507131800`

当前 Docker Compose 项目：

- Compose 项目名：`openhuge`
- 数据库容器：`openhuge-postgres-1`
- 控制面容器：`openhuge-control-plane-api-1`
- 公开反代容器：`openhuge-gateway-1`

端口模型：

- `control-plane-api` 在 Docker 内部监听 `8081`。
- 宿主机只把控制面暴露到 `127.0.0.1:18081`。
- 公开的 `gateway` 容器是 Nginx 反向代理，监听宿主机 `80` 端口，并代理到 `control-plane-api:8081`。
- 客户端应该连接 `http://175.27.228.168`，不要连接管理面板地址。

注意：当前服务器上的 `gateway` 容器不是仓库里的 `services/gateway-api`，而是当前 HugeCode 交付/控制面链路使用的 Nginx 公开反代。

不要默认认为仓库里的 `infra/docker/compose.yaml` 等同于服务器当前的 `/opt/openhuge/compose.yaml`。仓库里的文件是本地/runtime 基线；当前服务器是更小的自托管栈，主要由 Postgres、`control-plane-api`、Nginx `gateway` 组成。

不要不看内容就直接运行 `.codex-deploy` 里的旧脚本。部分脚本是在早期 bootstrap 阶段写的，可能会重写 `compose.yaml`，并把当前的 `gateway` 服务覆盖掉。

## 2. 敏感信息规则

以下内容禁止写进文档、issue、日志、客户端包或仓库：

- 面板路径、面板用户名、面板密码
- SSH 私钥
- `/opt/openhuge/.env`
- `CONTROL_PLANE_INTERNAL_TOKEN`
- `CONTROL_PLANE_CREDENTIAL_ENCRYPTION_KEY`
- Postgres 密码
- 本机 `openhuge-delivery.json` 里的 auth token
- 真实兑换码，除非是一次性 smoke 测试且明确要记录

公开客户端包只能内置非敏感运行信息，例如 API base URL。不能把生产方/operator token 打进公开包。

## 3. 部署前本地门禁

每次推服务器更新前，先在 `D:/OpenHuge` 跑对应本地门禁：

```powershell
pnpm verify:toolchain
node ./scripts/run-cargo.mjs fmt --check
node ./scripts/run-cargo.mjs check -p control-plane-api --locked
node ./scripts/run-cargo.mjs test -p control-plane-api --locked
```

如果本次改动涉及 gateway、ledger、protocol、schema、前端 client 合约，就必须扩展门禁。不要只跑 control-plane 测试就假装全链路没问题。

2026-05-07 这次 delivery server 修复至少通过了：

- `node ./scripts/run-cargo.mjs fmt --check`
- `node ./scripts/run-cargo.mjs test -p control-plane-api --locked stable_id_generation_does_not_reuse_request_sequence`
- `node ./scripts/run-cargo.mjs test -p control-plane-api --locked delivery_`
- `node ./scripts/run-cargo.mjs check -p control-plane-api --locked`

## 4. 构建源码包

默认部署来源必须是干净 worktree 的 `HEAD`：

```powershell
git status --short
git archive --format=tar.gz -o "D:/OpenHuge/.codex-deploy/openhuge-control-plane-src.tar.gz" HEAD
```

如果紧急部署必须包含未提交改动，先审查 diff，并在部署记录中明确说明。不要无声无息把本地 dirty 文件发到服务器。

上传到服务器后的目标路径：

```text
/tmp/openhuge-control-plane-src.tar.gz
```

上传方式取决于当前能用的 SSH 或面板能力。凭据必须留在仓库外。

## 5. 热更新：只更新 Control Plane 代码

适用场景：

- 只改 Rust 服务代码
- 不改 `compose.yaml`
- 不改 Nginx 配置
- 不改 volume 布局
- 不改密钥和环境变量合同

服务器上执行：

```bash
set -euo pipefail
cd /opt/openhuge

ts="$(date +%Y%m%d%H%M%S)"
mkdir -p "deploy-logs" "backups"

docker compose --env-file .env -f compose.yaml ps
curl -fsS http://127.0.0.1:18081/healthz
curl -fsS http://127.0.0.1/healthz

src_new="/opt/openhuge/source.new.${ts}"
mkdir -p "${src_new}"
tar -xzf /tmp/openhuge-control-plane-src.tar.gz -C "${src_new}"
rm -f /tmp/openhuge-control-plane-src.tar.gz

cp -a /opt/openhuge/source "/opt/openhuge/source-backup-${ts}"
mv /opt/openhuge/source "/opt/openhuge/source.prev.${ts}"
mv "${src_new}" /opt/openhuge/source

docker compose --env-file .env -f compose.yaml build control-plane-api 2>&1 | tee "deploy-logs/build-control-plane-${ts}.log"
docker compose --env-file .env -f compose.yaml up -d --no-deps control-plane-api

docker compose --env-file .env -f compose.yaml ps
curl -fsS http://127.0.0.1:18081/healthz
curl -fsS http://127.0.0.1/healthz
```

只有两个本地健康检查都通过，才算热更新成功。

如果 API 重启后公开反代异常，单独拉起 `gateway`：

```bash
cd /opt/openhuge
docker compose --env-file .env -f compose.yaml up -d gateway
docker compose --env-file .env -f compose.yaml logs --tail=80 gateway
curl -fsS http://127.0.0.1/healthz
```

## 6. 全量更新服务器

以下情况走全量更新：

- 修改 `/opt/openhuge/compose.yaml`
- 修改 Nginx 反代配置
- 修改暴露端口
- 修改环境变量合同
- 涉及有风险的数据库 schema 迁移
- 修改 Dockerfile 或基础运行模型

先做备份：

```bash
set -euo pipefail
cd /opt/openhuge
ts="$(date +%Y%m%d%H%M%S)"
mkdir -p backups deploy-logs

set -a
. ./.env
set +a

cp -a source "source-backup-${ts}"
cp -a compose.yaml "backups/compose-${ts}.yaml"
cp -a gateway "backups/gateway-${ts}" 2>/dev/null || true
docker compose --env-file .env -f compose.yaml exec -T postgres \
  pg_dump -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" -Fc \
  > "backups/postgres-${ts}.dump"
```

然后应用新源码/配置并重建：

```bash
cd /opt/openhuge
docker compose --env-file .env -f compose.yaml build control-plane-api gateway 2>&1 | tee "deploy-logs/build-full-${ts}.log"

# 只有本次发布明确包含数据库迁移时才执行。
docker compose --env-file .env -f compose.yaml run --rm control-plane-api migrate

docker compose --env-file .env -f compose.yaml up -d --remove-orphans
docker compose --env-file .env -f compose.yaml ps
```

禁止随手执行 `docker compose down -v`。这个命令会删除数据库 volume，属于破坏性操作，必须单独确认。

## 7. 部署后验证

最低健康门禁：

```bash
cd /opt/openhuge
docker compose --env-file .env -f compose.yaml ps
curl -fsS http://127.0.0.1:18081/healthz
curl -fsS http://127.0.0.1/healthz
curl -fsS http://175.27.228.168/healthz
```

期望返回 JSON，且 `status` 为 `ok`。

无密钥公开路径 smoke：

```bash
curl -sS -o /tmp/openhuge-redeem-invalid.out -w "%{http_code}\n" \
  -X POST http://127.0.0.1/v1/delivery-activations/redeem \
  -H 'content-type: application/json' \
  --data '{"activation_code":"invalid-smoke-code"}'
cat /tmp/openhuge-redeem-invalid.out
```

期望结果是受控业务错误，通常是 HTTP `400`。不能是超时、连接拒绝、反代 HTML 页面，或者容器崩溃。

完整 delivery smoke 需要 operator 凭据，必须从安全位置读取，不能写在本文档里。完整 smoke 应覆盖：

1. 创建或 prepare 一条 delivery
2. 上传并处理 artifact
3. 使用新兑换码 redeem
4. 通过返回的 grant 下载 artifact
5. 校验 artifact 字节数和 checksum

桌面客户端验证必须走真实构建客户端暴露的 bridge，不要用手写 HTTP 请求绕开客户端代码路径。

## 8. 回滚

如果热更新失败，并且还没有执行 schema 迁移：

```bash
set -euo pipefail
cd /opt/openhuge
ts="<failed_deploy_timestamp>"

rm -rf /opt/openhuge/source.failed."${ts}"
mv /opt/openhuge/source /opt/openhuge/source.failed."${ts}"
mv /opt/openhuge/source.prev."${ts}" /opt/openhuge/source

docker compose --env-file .env -f compose.yaml build control-plane-api
docker compose --env-file .env -f compose.yaml up -d --no-deps control-plane-api
curl -fsS http://127.0.0.1:18081/healthz
curl -fsS http://127.0.0.1/healthz
```

如果已经执行过数据库迁移，不能无脑回滚二进制。先判断旧二进制能不能读取新 schema。不能读取时，再从部署前 dump 恢复：

```bash
cd /opt/openhuge
docker compose --env-file .env -f compose.yaml exec -T postgres \
  pg_restore --clean --if-exists -U "${POSTGRES_USER}" -d "${POSTGRES_DB}" \
  < "backups/postgres-<timestamp>.dump"
```

## 9. 排障命令

常用服务器命令：

```bash
cd /opt/openhuge
docker compose --env-file .env -f compose.yaml ps
docker compose --env-file .env -f compose.yaml logs --tail=120 control-plane-api
docker compose --env-file .env -f compose.yaml logs --tail=120 gateway
ss -lntp | awk 'NR==1 || /:80 |:18081 |:37860 / {print}'
docker system df
df -h /
```

如果浏览器访问管理面板根 URL 一直卡住，优先检查完整面板 URL 和安全组。管理面板和 OpenHuge API 是两套东西；API 健康检查要看 `http://175.27.228.168/healthz`。

如果某一个点连续三次修不好，不要继续死磕同一个点，回到系统层面排查：

- 容器健康状态
- Nginx 反代路径
- Compose/env 是否漂移
- 源码包是否正确
- 数据库 schema 是否兼容
- 磁盘空间和 Docker build cache

## 10. 2026-05-07 更新记录

2026-05-07 这次更新修复了 `control-plane-api` 的持久化 ID 生成问题。故障模式是服务重启后继续使用 sequence 派生 ID，导致新请求和数据库已有记录冲突。修复方式是把持久化 ID 改为稳定随机 ID，并通过 targeted control-plane 测试和 delivery 路由 smoke 验证。

部署后已确认：

- 公开 `GET /healthz` 通过
- 内部完整 delivery smoke 通过
- 公开 delivery redeem/download 路径通过
- Windows 本机桌面客户端连接 `http://175.27.228.168` 的 redeem 路径通过

