FROM --platform=$TARGETPLATFORM alpine:3.16.2

COPY target/aarch64-unknown-linux-musl/release/mud ./mud
COPY data ./data

EXPOSE 8081

CMD ./mud
