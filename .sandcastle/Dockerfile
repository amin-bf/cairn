FROM node:22-bookworm

# Install system dependencies.
# build-essential / pkg-config / libssl-dev are here for Rust crates that
# compile C or link against system libraries (ring, openssl-sys, libsqlite3-sys
# without its bundled feature). The node:*-bookworm base is buildpack-deps
# based so it already carries a compiler, but naming them keeps the image
# correct if the base image is ever swapped for a slim variant.
RUN apt-get update && apt-get install -y \
  git \
  curl \
  jq \
  unzip \
  build-essential \
  pkg-config \
  libssl-dev \
  && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
  | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
  && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
  | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
  && apt-get update && apt-get install -y gh \
  && rm -rf /var/lib/apt/lists/*

# Build-args for UID/GID alignment: sandcastle docker build-image
# defaults these to the host user's UID/GID so image-built files
# and bind-mounted files share an owner without runtime chown.
ARG AGENT_UID=1000
ARG AGENT_GID=1000

# Rename the base image's "node" user to "agent" and align UID/GID.
RUN groupmod -o -g $AGENT_GID node && usermod -o -u $AGENT_UID -g $AGENT_GID -d /home/agent -m -l agent node
USER ${AGENT_UID}:${AGENT_GID}

# Install the Rust toolchain as the agent user, so cargo's home lands in
# /home/agent/.cargo and stays writable at runtime. clippy and rustfmt are
# requested explicitly because the minimal profile omits them, and the
# implement/review/merge prompts all run them as feedback loops.
# Note: --no-modify-path is deliberately NOT passed. The ENV PATH below covers
# the non-login shells that sandbox.exec uses, but letting rustup write its
# profile snippet means a login shell (bash -l) also finds cargo — otherwise
# /etc/profile resets PATH and the toolchain silently disappears.
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
  | sh -s -- -y --profile minimal \
  && /home/agent/.cargo/bin/rustup component add clippy rustfmt \
  && /home/agent/.cargo/bin/rustup target add aarch64-linux-android

# Give the agent a git identity. The host's ~/.gitconfig is not mounted into
# the sandbox and the repo carries no local user.*, so without this every
# `git commit` an agent attempts fails with "Author identity unknown" — and the
# implementer's whole contract is to produce commits.
RUN git config --global user.name "Sandcastle Agent" \
  && git config --global user.email "sandcastle@localhost" \
  && git config --global --add safe.directory /home/agent/workspace

# Install Claude Code CLI
RUN curl -fsSL https://claude.ai/install.sh | bash

# Add cargo and Claude to PATH
ENV PATH="/home/agent/.cargo/bin:/home/agent/.local/bin:$PATH"

# --- Android cross-compilation: compile and link, never package ---
#
# Without this the sandbox cannot so much as *compile* the
# `#[cfg(target_os = "android")]` arms, so any issue can break the Android build
# and nothing notices until someone builds by hand. With it,
# `cargo build --target aarch64-linux-android` works.
#
# rustup's target alone is not enough, and this is the part that surprises:
# `cargo check` runs build scripts, and rusqlite's `bundled` feature compiles
# SQLite's C with `cc`, so a cross C toolchain is needed even to type-check.
#
# Deliberately omitted: the JDK, the SDK platforms and Gradle. `cargo apk build`
# therefore does NOT run here, and packaging regressions — the emitted manifest,
# the intent filters — stay a human check on the handset. Adding them would
# roughly double the image for a check that still could not answer the questions
# a device answers.
#
# Pinned to the NDK the host carries (29.0.13846066, r29-beta3, clang 21.0.0) so
# a failure here means a real failure rather than a toolchain difference. The
# Android findings in AGENTS.md — `--link-builtins` not needed, 16 KB page
# alignment working out of the box — were established against this one.
# Installed by direct download rather than `sdkmanager`, which is a Java program
# and would drag in the JDK this whole tier exists to avoid.
#
# Costs ~0.73 GB to fetch and ~4 GB in the image. It is one layer, shared by
# every sandbox, so it is paid once rather than per parallel pipeline.
ARG ANDROID_NDK_RELEASE=r29-beta3
ENV ANDROID_NDK_HOME=/home/agent/android-ndk
RUN curl -fsSL -o /tmp/ndk.zip \
  "https://dl.google.com/android/repository/android-ndk-${ANDROID_NDK_RELEASE}-linux.zip" \
  && unzip -q /tmp/ndk.zip -d /home/agent \
  && mv "/home/agent/android-ndk-${ANDROID_NDK_RELEASE}" "${ANDROID_NDK_HOME}" \
  && rm /tmp/ndk.zip

# The API level in the compiler wrapper's name must match `min_sdk_version` in
# crates/app/Cargo.toml. Raise one and the other stops being true silently: the
# build keeps succeeding and the artifact stops running on the oldest handset it
# claims to support.
ENV ANDROID_NDK_ROOT="${ANDROID_NDK_HOME}" \
  NDK_HOME="${ANDROID_NDK_HOME}" \
  PATH="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin:${PATH}" \
  CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=aarch64-linux-android24-clang \
  CC_aarch64_linux_android=aarch64-linux-android24-clang \
  AR_aarch64_linux_android=llvm-ar

WORKDIR /home/agent

# In worktree sandbox mode, Sandcastle bind-mounts the git worktree at /home/agent/workspace
# and overrides the working directory to /home/agent/workspace at container start.
# Structure your Dockerfile so that /home/agent/workspace can serve as the project root.
ENTRYPOINT ["sleep", "infinity"]
