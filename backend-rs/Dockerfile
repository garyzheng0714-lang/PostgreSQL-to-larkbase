# 多阶段构建：musl 静态二进制 → 极小运行镜像（设计 §6.6）
# 产物为单文件静态二进制，最终镜像仅含二进制 + CA 证书，约 10MB。

# ---- 构建阶段 ----
FROM rust:1-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
# 先拷依赖清单，利用缓存层
COPY Cargo.toml Cargo.lock ./
# 拷源码并构建（release：lto + 单 codegen-unit + strip，见 Cargo.toml）
COPY src ./src
RUN cargo build --release --locked \
    && cp target/release/fbif-databridge /fbif-databridge

# ---- 运行阶段 ----
FROM alpine:3
# CA 证书（rustls verify-full 需要；scratch 无证书故用含 ca 的 alpine）
RUN apk add --no-cache ca-certificates tzdata \
    && adduser -D -u 10001 app
USER app
COPY --from=builder /fbif-databridge /usr/local/bin/fbif-databridge
ENV BIND_ADDR=0.0.0.0:18082 \
    RUST_LOG=info
EXPOSE 18082
# 健康检查走 /health
HEALTHCHECK --interval=30s --timeout=3s --retries=3 \
    CMD wget -qO- http://127.0.0.1:18082/health || exit 1
ENTRYPOINT ["/usr/local/bin/fbif-databridge"]
