# PostgreSQL-to-larkbase

把 PostgreSQL 表或自定义 SQL 结果同步到飞书多维表格的自定义连接器。

这个项目包含两部分：

- 前端配置页：在多维表格里弹出，供用户填写数据库连接、选择表或 SQL、配置字段映射
- 后端连接器：对外提供 `meta.json`、`/api/table_meta`、`/api/records` 和前端辅助接口

适合的使用方式是：把它部署到你自己的服务器，然后在多维表格里以“自定义连接器 / 自助连接器”的形式接入。

## 功能概览

- 支持 PostgreSQL 表、视图、自定义 SQL
- 支持字段选择、字段重命名、数字精度配置
- 支持自动同步开关
- 支持请求签名校验
- 支持多维表格常见字段类型映射
- 通过 `meta.json` 暴露连接器协议

当前实现的主要限制：

- 单次同步最多返回 `50000` 行，可通过 `MAX_ROW_LIMIT` 调整
- 字段数最多 `299`
- 自动同步描述为每小时一次，实际调度由多维表格侧控制
- 未识别的 PostgreSQL 类型会退化成文本

## 目录结构

```text
backend/   FastAPI 后端连接器
frontend/  React + Vite 前端配置页
deploy.sh  当前仓库作者服务器的部署脚本示例
```

## 运行原理

1. 多维表格读取 `https://你的域名/meta.json`
2. `meta.json` 告诉多维表格：
   - 前端配置页地址 `dataSourceConfigUiUri`
   - 拉取表结构的接口 `/api/table_meta`
   - 拉取记录的接口 `/api/records`
3. 用户在配置页里填写 PostgreSQL 信息并保存
4. 多维表格随后带着 `datasourceConfig` 调用后端接口，完成建表和拉数

## 部署前准备

至少准备以下内容：

- 一台能被公网访问的 Linux 服务器
- 一个已经解析到该服务器的域名
- HTTPS 证书
- Docker 与 Docker Compose
- Nginx 或其他能托管静态文件并做反向代理的 Web 服务器
- 一个有读取权限的 PostgreSQL 账号

建议：

- 为同步专门创建只读账号
- 让 PostgreSQL 只放通服务器出口 IP
- 生产环境不要使用默认 `testBase`

## 推荐部署拓扑

最省事的生产方案是同域名部署：

- `https://pg2base.example.com/`：前端静态页面
- `https://pg2base.example.com/meta.json`：后端元数据
- `https://pg2base.example.com/api/*`：后端接口

这样前端里的 `VITE_API_BASE_URL` 可以留空，浏览器会自动同域请求 `/api/helper/*`。

## 方式一：部署到单台服务器

### 1. 拉取代码

```bash
git clone https://github.com/garyzheng0714-lang/PostgreSQL-to-larkbase.git
cd PostgreSQL-to-larkbase
```

### 2. 配置后端环境变量

新建 `backend/.env.production`：

```env
SECRET_KEY=replace-with-a-long-random-secret
FRONTEND_URL=https://pg2base.example.com/
BACKEND_URL=https://pg2base.example.com
CORS_ORIGINS=["https://pg2base.example.com","https://feishu.cn","https://www.feishu.cn"]
PG_CONNECT_TIMEOUT=5
PG_QUERY_TIMEOUT=15
MAX_ROW_LIMIT=50000
LOG_LEVEL=INFO
```

说明：

- `SECRET_KEY`：要和多维表格里配置的连接器签名密钥保持一致
- `FRONTEND_URL`：必须是公网可访问的前端地址，`meta.json` 会返回它
- `CORS_ORIGINS`：至少包含你的前端域名；如果是飞书中国区，保留 `feishu.cn` 即可
- `BACKEND_URL`：当前代码里还没实际消费，但建议保持为真实公网地址

### 3. 构建前端静态文件

同域名部署时，直接使用相对路径访问后端：

```bash
cd frontend
npm ci
VITE_API_BASE_URL="" npm run build
```

构建完成后得到 `frontend/dist/`。

### 4. 启动后端

仓库自带的 `docker-compose.yml` 可以直接用于后端容器启动：

```bash
cd /path/to/PostgreSQL-to-larkbase
docker compose up -d --build
```

注意：当前仓库里的 [docker-compose.yml](/Users/simba/local_vibecoding/PostgreSQL-to-larkbase/docker-compose.yml) 使用了外部网络 `postgres_default`。如果你的服务器上没有这个网络，需要二选一：

- 删除 `services.backend.networks` 和底部 `networks` 配置，使用默认 bridge 网络
- 或手动创建同名外部网络

如果你不确定，通常删掉这段网络配置最简单。

### 5. 配置 Nginx

下面是一个同域名部署示例：

```nginx
server {
    listen 80;
    server_name pg2base.example.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name pg2base.example.com;

    ssl_certificate /etc/letsencrypt/live/pg2base.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/pg2base.example.com/privkey.pem;

    root /var/www/pg2base;
    index index.html;

    location / {
        try_files $uri /index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:18082;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location = /meta.json {
        proxy_pass http://127.0.0.1:18082;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location = /health {
        proxy_pass http://127.0.0.1:18082;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

把前端构建产物复制到 Nginx 静态目录：

```bash
mkdir -p /var/www/pg2base
cp -R frontend/dist/* /var/www/pg2base/
nginx -t && systemctl reload nginx
```

### 6. 验证部署

至少检查这几个地址：

```bash
curl -sSf https://pg2base.example.com/health
curl -sSf https://pg2base.example.com/meta.json | jq
```

你应该看到：

- `/health` 返回 `{"status":"ok"}`
- `/meta.json` 返回 `type: "data_connector"`，并且 `dataSourceConfigUiUri` 指向你的前端地址

## 方式二：前后端分域名部署

如果前端和后端不在同一个域名，也可以用：

- 前端：`https://pg2base-ui.example.com/`
- 后端：`https://pg2base-api.example.com/`

此时需要：

1. 构建前端时显式指定 API 地址

```bash
VITE_API_BASE_URL="https://pg2base-api.example.com" npm run build
```

2. 后端环境变量改成：

```env
FRONTEND_URL=https://pg2base-ui.example.com/
CORS_ORIGINS=["https://pg2base-ui.example.com","https://feishu.cn","https://www.feishu.cn"]
```

3. 多维表格读取的 `meta.json` 仍然来自后端域名

## 接入多维表格

多维表格侧的文案和入口可能会随版本变化，但大致流程是一样的：

1. 打开一个多维表格
2. 找到“从外部数据导入 / 数据连接器 / 自定义连接器”之类的入口
3. 填入你的连接器地址：

```text
https://pg2base.example.com/meta.json
```

4. 如果界面要求填写请求签名密钥，填写与 `SECRET_KEY` 相同的值
5. 创建成功后，打开连接器配置页
6. 在配置页里依次完成：
   - 数据库地址、端口、用户名、密码、数据库名
   - 选择 Schema + 表 / 视图，或改用自定义 SQL
   - 选择同步字段、重命名字段、配置数值精度
   - 决定是否开启自动同步
7. 保存后，多维表格会调用：
   - `/api/table_meta` 获取字段结构
   - `/api/records` 拉取记录

如果一切正常，你会在多维表格中看到新建出来的同步表。

## PostgreSQL 权限建议

建议只给同步账号下面这些能力：

- `CONNECT` 到目标数据库
- `USAGE` 到目标 schema
- `SELECT` 目标表或视图

示例：

```sql
CREATE USER bitable_sync WITH PASSWORD 'replace_me';
GRANT CONNECT ON DATABASE your_db TO bitable_sync;
GRANT USAGE ON SCHEMA public TO bitable_sync;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO bitable_sync;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT SELECT ON TABLES TO bitable_sync;
```

## 字段类型映射

当前内置映射大致如下：

- 文本：`text`、`varchar`、`uuid`、`json/jsonb`、数组和大部分未知类型
- 数字：`int`、`bigint`、`numeric`、`decimal`、`serial`
- 日期：`date`、`timestamp`、`timestamptz`
- 复选框：`bool`
- 货币：`money`

说明：

- `time` 和时区时间目前会按文本处理
- 数组会按文本处理
- `json/jsonb` 会序列化成字符串

## 常见排障

### 配置页能打开，但测试连接失败

优先检查：

- PostgreSQL 地址、端口、防火墙
- 数据库名是否存在
- 用户名密码是否正确
- 账号是否有 `SELECT` 权限

### 配置页打不开或是空白

优先检查：

- `FRONTEND_URL` 是否为公网 HTTPS 地址
- 前端静态文件是否真的部署到了 Nginx
- 如果前后端分域名，`CORS_ORIGINS` 是否包含前端域名

### 多维表格提示签名失败

优先检查：

- 多维表格里配置的签名密钥是否和 `SECRET_KEY` 一致
- 反向代理是否篡改了请求体
- 生产环境是否还在使用默认 `testBase`

### `docker compose up` 报网络不存在

这是因为仓库示例的 [docker-compose.yml](/Users/simba/local_vibecoding/PostgreSQL-to-larkbase/docker-compose.yml) 依赖外部网络 `postgres_default`。大多数自部署场景可以直接删掉这段网络配置。

### 自定义 SQL 预览失败

优先检查：

- SQL 语法是否正确
- SQL 是否引用了当前账号无权限访问的对象
- SQL 返回字段数是否过多
- SQL 执行是否超时

## 本地开发

### 前端

```bash
cd frontend
npm ci
npm run dev
```

### 后端

项目要求 Python `>= 3.11`。仓库中的 Docker 镜像使用的是 Python `3.12`。

```bash
cd backend
uv sync --dev
uv run uvicorn src.main:app --reload --host 0.0.0.0 --port 8000
```

本地开发时可以参考 [backend/.env.example](/Users/simba/local_vibecoding/PostgreSQL-to-larkbase/backend/.env.example)。

## 生产建议

- 使用 HTTPS，不要裸露 HTTP
- 使用只读数据库账号
- 把 `SECRET_KEY` 换成随机长字符串
- 用防火墙限制数据库来源 IP
- 为 Nginx 和容器日志做轮转
- 针对大表优先使用视图或自定义 SQL 做裁剪

## 当前仓库里几个需要注意的文件

- [deploy.sh](/Users/simba/local_vibecoding/PostgreSQL-to-larkbase/deploy.sh)：这是当前作者机器上的部署脚本示例，不是通用的一键安装脚本
- [docker-compose.yml](/Users/simba/local_vibecoding/PostgreSQL-to-larkbase/docker-compose.yml)：能直接起后端，但网络配置默认偏作者环境
- [backend/.env.example](/Users/simba/local_vibecoding/PostgreSQL-to-larkbase/backend/.env.example)：开发环境变量示例

## 下一步可做的增强

如果你准备把这个连接器长期给团队使用，建议继续补这些能力：

- 在后端直接托管前端静态文件，减少 Nginx 配置复杂度
- 增加更完整的生产版 `docker-compose.yml`
- 增加鉴权白名单和访问日志脱敏
- 增加 SQL 安全限制和只读校验
- 增加多租户隔离与监控告警
