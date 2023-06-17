#########################
# Build agent binary #
#########################
ARG RUST_VERSION=1.70.0-bookworm
FROM rust:$RUST_VERSION as builder

# Add the code and compile.
COPY . /code
RUN cargo build --manifest-path /code/Cargo.toml --release --locked


######################################
# Package agent into a smaller image #
######################################
FROM debian:bookworm-slim

# Create a replicante user to avoid using root.
ARG REPLI_GID=1616
ARG REPLI_GNAME=replicante
ARG REPLI_UID=1616
ARG REPLI_UNAME=replicante
RUN addgroup --gid $REPLI_GID $REPLI_GNAME \
    && adduser --disabled-login --disabled-password --system \
        --uid $REPLI_UID --gid $REPLI_GID $REPLI_UNAME

# Install needed runtime dependencies.
RUN DEBIAN_FRONTEND=noninteractive apt-get update \
    && apt-get install -y libssl3 \
    && apt-get clean all

# Copy binary from builder to smaller image.
COPY --from=builder /code/target/release/repliagent-mongodb /opt/replicante/bin/repliagent-mongodb

# Set up runtime environment as needed.
ENV PATH=/opt/replicante/bin:$PATH
USER $REPLI_UNAME
WORKDIR /home/replicante
CMD ["/opt/replicante/bin/repliagent-mongodb"]

# Validate binary.
RUN /opt/replicante/bin/repliagent-mongodb --version
