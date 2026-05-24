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

echo "Fetching fonts (not tracked in git)"
bash /workspaces/ohmyoled/scripts/fetch-fonts.sh

echo "Copying fonts over"
cp -Rv /workspaces/ohmyoled/fonts/* /usr/share/fonts/

echo "Installing Claude Code"
npm install -g @anthropic-ai/claude-code

# Restore exec bit on the workspace-local Claude statusline script. The
# script + its config live in /workspaces/ohmyoled/.claude/ so they survive
# devcontainer rebuilds (the workspace is volume-mounted); the +x bit can
# be lost when the host filesystem doesn't preserve POSIX modes.
if [ -f /workspaces/ohmyoled/.claude/statusline-command.sh ]; then
    chmod +x /workspaces/ohmyoled/.claude/statusline-command.sh
    echo "Restored exec bit on .claude/statusline-command.sh"
fi
