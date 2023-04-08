FROM rust:1.68 as build

WORKDIR /build

COPY ./ ./
RUN cargo build --release

FROM rust:1.68

WORKDIR /app

RUN apt update && apt install -y ca-certificates
COPY --from=build /build/target/release/telegram-bot-torrents /bin/telegram-bot-torrents

CMD ["/bin/telegram-bot-torrents"]
