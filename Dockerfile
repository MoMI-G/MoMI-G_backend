FROM buildpack-deps:bionic as build

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUSTFLAGS="-C link-arg=-fuse-ld=lld -C target-feature=-avx2 -C target-feature=-avx512f -C target-feature=-avx512dq -C target-feature=-avx512vl -C target-feature=-xsavec -C target-feature=-avx512bw -C target-feature=-avx512cd -C target-feature=-xsaveopt"

RUN grep sse /proc/cpuinfo

RUN set -eux; \
    \
    url="https://static.rust-lang.org/rustup/dist/x86_64-unknown-linux-gnu/rustup-init"; \
    wget "$url"; \
    chmod +x rustup-init; \
    ./rustup-init -y --no-modify-path --default-toolchain nightly; \
    rm rustup-init; \
    chmod -R a+w $RUSTUP_HOME $CARGO_HOME; \
    mkdir /usr/src/app;

RUN apt-get update && apt-get install -y \
		cmake clang python-pip lld \
	&& rm -rf /var/lib/apt/lists/*

RUN pip install cmake

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release; \
    rm -rf src/;

# ---------------------------
FROM quay.io/vgteam/vg:v1.25.0

ARG BUILD_DATE
ARG VCS_REF

LABEL org.label-schema.build-date=$BUILD_DATE \
      org.label-schema.vcs-ref=$VCS_REF \
      org.label-schema.vcs-url="https://github.com/MoMI-G/MoMI-G/" \
      org.label-schema.schema-version="1.0.0"

# Add dependency
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -

RUN apt-get update && apt-get install -y \
		ruby nodejs git \
	&& rm -rf /var/lib/apt/lists/*

WORKDIR /build

RUN npm install --global yarn && git clone https://github.com/MoMI-G/MoMI-G && cd MoMI-G && yarn && yarn build && cp -r build /vg/build

# Create app directory
WORKDIR /vg

RUN mkdir /vg/tmp; \
    mkdir /vg/tmp/xg; \
    mkdir /vg/sample;

# ADD static /vg/static
ADD proto/graph-helper* /vg/proto/
COPY --from=build /usr/src/app/target/release/graph-genome-browser-backend /vg/

EXPOSE 8081

ENV RUST_LOG info,graph_genome_browser_backend=debug

CMD ["./graph-genome-browser-backend", "--config=static/config.yaml", "--interval=1500000", "--http=0.0.0.0:8081", "--api=/api/v2/", "--serve", "--build=build"]
