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
  && /home/agent/.cargo/bin/rustup component add clippy rustfmt

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

WORKDIR /home/agent

# In worktree sandbox mode, Sandcastle bind-mounts the git worktree at /home/agent/workspace
# and overrides the working directory to /home/agent/workspace at container start.
# Structure your Dockerfile so that /home/agent/workspace can serve as the project root.
ENTRYPOINT ["sleep", "infinity"]
