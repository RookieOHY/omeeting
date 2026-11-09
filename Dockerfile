# ---------- 阶段 1：构建前端 ----------
FROM node:20-alpine AS web-build
WORKDIR /build
COPY web/package.json web/package-lock.json ./
RUN npm ci || npm install
COPY web/ .
RUN npm run build
# 产物在 /build/dist

# ---------- 阶段 2：构建 Rust 服务（前端 dist 编译进二进制）----------
FROM rust:1.93-slim AS rust-build
WORKDIR /build
RUN apt-get update && apt-get install -y pkg-config && rm -rf /var/lib/apt/lists/*
COPY server/Cargo.toml server/Cargo.lock* ./
# 先拷依赖清单做层缓存
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src
COPY server/ .
COPY --from=web-build /build/dist web/dist
RUN touch src/main.rs && cargo build --release

# ---------- 阶段 3：运行时 ----------
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=rust-build /build/target/release/omeeting-server /usr/local/bin/omeeting-server
EXPOSE 7070 7071/udp
ENV RUST_LOG=omeeting_server=info,str0m=warn
ENTRYPOINT ["/usr/local/bin/omeeting-server"]
