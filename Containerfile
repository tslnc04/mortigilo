FROM rust:1-alpine AS builder

WORKDIR /app

RUN apk add --no-cache musl-dev
RUN rustup target add x86_64-unknown-linux-musl

COPY . ./
RUN cargo install --target x86_64-unknown-linux-musl --path .

FROM scratch AS mortigilo

COPY --from=builder /usr/local/cargo/bin/mortigilo /mortigilo
USER 1000:1000

ENTRYPOINT ["/mortigilo"]
