FROM rust:1.75-bookworm

# Install basic development tools
RUN apt update && apt install -y \
    git \
    curl \
    pkg-config \
    libssl-dev \
    build-essential \
    redis-server \
    && rm -rf /var/lib/apt/lists/*

# Install Rust nightly and required components
RUN rustup toolchain install nightly --component llvm-tools-preview rustfmt clippy \
    && rustup override set nightly

# Install cargo tools
RUN cargo install cargo-deny \
    && cargo install cargo-audit \
    && cargo install grcov \
    && cargo install cargo-watch

# Install Task
RUN sh -c "$(curl --location https://taskfile.dev/install.sh)" -- -d -b /usr/local/bin

# Create a non-root user
ARG USERNAME=dev
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && apt-get update \
    && apt-get install -y sudo \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

USER $USERNAME

# Set environment variables
ENV RUST_BACKTRACE=1
ENV RUST_LOG=debug

# Set working directory
WORKDIR /workspace 