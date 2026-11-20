# Build context: the repository root.
ARG BUILD_FROM=ghcr.io/home-assistant/amd64-base:3.21

FROM rust:1.91-alpine AS build
RUN apk add --no-cache curl musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked && strip target/release/tinyasr-wyoming
# the model files come from the tinyasr commit Cargo.lock pins
RUN rev=$(sed -n 's|.*nemethhh/tinyasr?branch=wyoming-pause-end#\([0-9a-f]*\)".*|\1|p' Cargo.lock) \
    && test -n "$rev" && mkdir -p /out/models \
    && for f in en-ru-uk.tzf silero_vad.onnx; do \
         curl -fsSL "https://raw.githubusercontent.com/nemethhh/tinyasr/$rev/models/$f.gz" | gunzip > "/out/models/$f"; \
       done \
    && echo "$rev" > /out/models/REV

FROM $BUILD_FROM
COPY --from=build /src/target/release/tinyasr-wyoming /usr/bin/tinyasr-wyoming
COPY --from=build /out/models /models
COPY tinyasr/run.sh /run.sh
RUN chmod a+x /run.sh
LABEL org.opencontainers.image.source="https://github.com/nemethhh/tinyasr-wyoming" \
      org.opencontainers.image.description="tinyasr speech to text for Home Assistant (Wyoming)" \
      org.opencontainers.image.licenses="GPL-3.0-or-later"
CMD ["/run.sh"]
