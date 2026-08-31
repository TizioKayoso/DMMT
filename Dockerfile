FROM rust:slim AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/app/target/release/minecraft_map /app/minecraft_map
COPY --from=builder /usr/src/app/public /app/public
RUN mkdir -p /app/region /app/tiles /app/tiles_height
EXPOSE 8080
CMD [ "/app/minecraft_map" ]
