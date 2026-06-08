#!/bin/bash

set -euo pipefail

## Update apt
echo "Installing any apt upgrades"
apt-get update && apt-get upgrade -y

## Install necessary packages
echo "Installing needed packages"
apt-get install -y \
    fonts-noto-mono \
    vim \
    shellcheck \
    build-essential

## Rust toolchain components. rust-analyzer is the Rust LSP binary that the
## Claude Code rust-analyzer-lsp plugin (wired up below) drives from PATH.
## clippy + rustfmt back the verification loop in CLAUDE.md. The base image
## ships these today, but pin them explicitly so a base bump can't drop them.
## `rustup component add` is idempotent — a no-op if already present.
echo "Ensuring Rust LSP + components (rust-analyzer, clippy, rustfmt)"
rustup component add rust-analyzer clippy rustfmt

echo "Fetching fonts (not tracked in git)"
bash /workspaces/ohmyoled/scripts/fetch-fonts.sh

echo "Copying fonts over"
cp -Rv /workspaces/ohmyoled/fonts/* /usr/share/fonts/

echo "Installing Claude Code"
npm install -g @anthropic-ai/claude-code

## Give Claude Code the Rust language server (rust-analyzer) via the official
## plugin. The marketplace + enabled-plugin entries are committed in
## .claude/settings.json (project scope), so this just pre-fetches the plugin
## files into the container at create time — without it Claude would fetch them
## on first run instead. Public GitHub fetch, no auth; tolerate offline/forks.
echo "Installing the Rust LSP plugin for Claude Code (rust-analyzer-lsp)"
claude plugin marketplace add anthropics/claude-plugins-official --scope project 2>/dev/null || true
claude plugin install rust-analyzer-lsp@claude-plugins-official --scope project 2>/dev/null \
    || echo "WARNING: could not pre-install rust-analyzer-lsp; Claude will install it on first run from .claude/settings.json"

# Restore exec bit on the workspace-local Claude statusline script. The
# script + its config live in /workspaces/ohmyoled/.claude/ so they survive
# devcontainer rebuilds (the workspace is volume-mounted); the +x bit can
# be lost when the host filesystem doesn't preserve POSIX modes.
if [ -f /workspaces/ohmyoled/.claude/statusline-command.sh ]; then
    chmod +x /workspaces/ohmyoled/.claude/statusline-command.sh
    echo "Restored exec bit on .claude/statusline-command.sh"
fi
