#!/bin/bash

set -euo pipefail

RGBMATRIX_GIT_URL="https://github.com/hzeller/rpi-rgb-led-matrix.git"

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

python3 -m pip install "Cython>=3.0"

echo "Clone the RGBMatrix"
if [ ! -d /tmp/rpi-rgb-led-matrix ]; then
    git clone "$RGBMATRIX_GIT_URL" /tmp/rpi-rgb-led-matrix
fi

# Build RGB Matrix
echo "Install RGB Matrix"
cd /tmp/rpi-rgb-led-matrix && make build-python PYTHON="$(which python3)" && make install-python PYTHON="$(which python3)" && cd bindings && python3 -m pip install -e python/ -I

CFLAGS=-fcommon python3 -m pip install Pyinstaller RPi.GPIO
CFLAGS=-fcommon python3 -m pip install -e /workspaces/ohmyoled/src/python/

echo "Copying fonts over"
cp -Rv /workspaces/ohmyoled/fonts/* /usr/share/fonts/

echo "Installing Claude Code"
npm install -g @anthropic-ai/claude-code
