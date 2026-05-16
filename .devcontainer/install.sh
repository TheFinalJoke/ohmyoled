#!/bin/bash

set -euo pipefail

## Update apt
echo "Installing any apt upgrades"
apt-get update && apt-get upgrade -y

## Install necessary packages
echo "Installing needed packages"
apt-get install -y \
    fonts-noto-mono \
    python3.12 \
    python3.12-dev \
    python3-pip \
    python3.12-venv \
    vim \
    shellcheck \
    build-essential

# Point python3 at 3.12
update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.12 1

# Upgrade Pip to latest version
echo "Upgrading pip"
python3 -m pip install --upgrade pip

python3 -m pip install "Cython>=3.0" maturin

# Build and install the ohmyoled-matrix Python bindings (replaces the old rgbmatrix clone)
echo "Building ohmyoled-matrix Python bindings"
cd /workspaces/ohmyoled/crates/ohmyoled-matrix-py && maturin build --release
pip3 install --force-reinstall /workspaces/ohmyoled/target/wheels/ohmyoled_matrix-*.whl

CFLAGS=-fcommon python3 -m pip install Pyinstaller RPi.GPIO
CFLAGS=-fcommon python3 -m pip install -e /workspaces/ohmyoled/src/python/

echo "Copying fonts over"
cp -Rv /workspaces/ohmyoled/fonts/* /usr/share/fonts/

echo "Installing Claude Code"
npm install -g @anthropic-ai/claude-code
