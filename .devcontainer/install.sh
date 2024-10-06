#!/bin/bash

set -euo pipefail

OLED_VERSION="2.2.8"

RGBMATRIX_GIT_URL="https://github.com/hzeller/rpi-rgb-led-matrix.git"

## Update apt 
echo "Installing any apt upgrades"
apt update && apt upgrade -y 

## Install nessccary packages 
echo "Installing any needed packages"
apt install fonts-noto-mono python3-dev cython3 -y 

# Upgrade Pip to latest version
echo "Upgrading pip"
python3 -m pip install --upgrade pip

python3 -m pip install Cython

echo "Clone the RGBMatrix"
if [ ! -d /tmp/rpi-rgb-led-matrix ];
then
    git clone $RGBMATRIX_GIT_URL /tmp/rpi-rgb-led-matrix
fi

# Build RGB Matrix 
echo "Install RGB Matrix"
cd /tmp/rpi-rgb-led-matrix && make build-python PYTHON="$(which python3)" && make install-python PYTHON="$(which python3)" && cd bindings && python3 -m pip install -e python/ -I
export CFLAGS=-fcommon && python3 -m pip install Pyinstaller RPi.GPIO
export CFLAGS=-fcommon && python3 -m pip install ohmyoled==${OLED_VERSION}

echo "Copying fonts over"
cp -Rv /workspaces/ohmyoled/fonts/* /usr/share/fonts/