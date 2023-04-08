FROM rust:1.49

COPY ./ ./

RUN cargo build --release
RUN apt update && apt install -y ca-certificates

CMD ["./target/release/telegram-bot-torrents"]
