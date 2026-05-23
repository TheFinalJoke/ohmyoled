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

echo "Copying fonts over"
cp -Rv /workspaces/ohmyoled/fonts/* /usr/share/fonts/

echo "Installing Claude Code"
npm install -g @anthropic-ai/claude-code
