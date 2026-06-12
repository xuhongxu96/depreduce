FROM rust:1.91.0-bookworm

ARG BAZEL_VERSION=8.3.1
ARG BUCK2_URL=https://github.com/facebook/buck2/releases/download/latest/buck2-x86_64-unknown-linux-gnu.zst

ENV DEBIAN_FRONTEND=noninteractive
ENV USE_BAZEL_VERSION=${BAZEL_VERSION}
ENV PATH=/artifact/depreduce/target/release:/root/.cargo/bin:/usr/local/bin:${PATH}

RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    build-essential \
    ca-certificates \
    curl \
    git \
    gnupg \
    openjdk-17-jdk \
    pkg-config \
    python3 \
    python3-pip \
    strace \
    unzip \
    wget \
    zstd \
    && rm -rf /var/lib/apt/lists/*

RUN wget -O /usr/local/bin/bazelisk \
    https://github.com/bazelbuild/bazelisk/releases/download/v1.26.0/bazelisk-linux-amd64 \
    && chmod +x /usr/local/bin/bazelisk \
    && ln -s /usr/local/bin/bazelisk /usr/local/bin/bazel \
    && bazel --version

RUN wget -O /tmp/buck2.zst "${BUCK2_URL}" \
    && unzstd /tmp/buck2.zst -o /usr/local/bin/buck2 \
    && chmod +x /usr/local/bin/buck2 \
    && rm /tmp/buck2.zst \
    && buck2 --version

RUN python3 -m pip install --break-system-packages --no-cache-dir \
    matplotlib \
    numpy \
    pandas \
    scipy \
    seaborn

WORKDIR /artifact/depreduce
COPY . .

RUN rm -rf buildfuzz strace_parser data/strace_parser

RUN perl -0pi -e 's/members = \[[^\]]+\]/members = ["depreduce", "depstat", "utils"]/s' Cargo.toml

RUN p1="$(printf '\170\165\150\157\156\147\170\165\071\066')" \
    && p2="$(printf '\110\157\156\147\170\165\040\130\165')" \
    && p3="$(printf '\150\157\156\147\170\165')" \
    && p4="$(printf '\150\064\064\065\170\165')" \
    && p5="$(printf '\165\167\141\164\145\162\154\157\157')" \
    && p6="$(printf '\057\144\141\164\141\057\150\064\064\065\170\165')" \
    && p7="$(printf '\057\150\157\155\145\057\150\157\156\147\170\165')" \
    && p8="$(printf '\137\142\141\172\145\154\137\150\157\156\147\170\165')" \
    && pat="${p1}|${p2}|${p3}|${p4}|${p5}|${p6}|${p7}|${p8}|github[.]com/[^[:space:])]+/pull/[0-9]+|[P]R[[:space:]]+[0-9]+|pull[[:space:]]+request[[:space:]]+[0-9]+" \
    && find . -type f \
        ! -path './target/*' \
        ! -path './.git/*' \
        ! -name '*.pdf' \
        ! -name '*.gz' \
        ! -name '*.sha256' \
        -print0 \
    | xargs -0r grep -IlZ -E "$pat" \
    | xargs -0r perl -pi -e 'BEGIN { @r = (["\x78\x75\x68\x6f\x6e\x67\x78\x75\x39\x36","anonymous"], ["\x48\x6f\x6e\x67\x78\x75\x20\x58\x75","Artifact Author"], ["\x68\x6f\x6e\x67\x78\x75","user"], ["\x68\x34\x34\x35\x78\x75","artifact"], ["\x75\x77\x61\x74\x65\x72\x6c\x6f\x6f","example"], ["\x2f\x64\x61\x74\x61\x2f\x68\x34\x34\x35\x78\x75","/artifact"], ["\x2f\x68\x6f\x6d\x65\x2f\x68\x6f\x6e\x67\x78\x75","/home/user"], ["\x5f\x62\x61\x7a\x65\x6c\x5f\x68\x6f\x6e\x67\x78\x75","_bazel_user"]); } for my $r (@r) { s/\Q$r->[0]\E/$r->[1]/gi; } s{https://github[.]com/[^\s)]+/pull/[0-9]+}{[upstream change]}g; s{\b[P]R\s+[0-9]+}{upstream change}g; s{pull\s+request\s+[0-9]+}{upstream change}gi' \
    && find . -type f \( -name '*.stdout' -o -name '*.stderr' -o -name '*.pdf' \) -delete

RUN cargo build -p depreduce -p depstat --bins --release \
    && cp /artifact/depreduce/target/release/depreduce /usr/local/bin/depreduce \
    && cp /artifact/depreduce/target/release/depstat /usr/local/bin/depstat \
    && ln -sf /usr/local/cargo/bin/cargo /usr/local/bin/cargo \
    && ln -sf /usr/local/cargo/bin/rustc /usr/local/bin/rustc \
    && ln -sf /usr/local/cargo/bin/rustup /usr/local/bin/rustup \
    && rm -rf /artifact/depreduce/target /usr/local/cargo/registry /usr/local/cargo/git

CMD ["/bin/bash"]
