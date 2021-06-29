#!/bin/bash

if [[ `id -u` != 0]]
then
echo "Become Root"
exit 2
fi
apt update && apt upgrade -y

echo "It Could take 10-20 Mins"
apt install -y build-essential git python3-setuptools python3-pip python3-dev python3-pillow python3-numpy python3-gpiozero python3-cairosvg libatlas3-base libatlas-base-dev libraqm-dev jq pastebinit neofetch zsh dbus

pip3 install -r requirements.txt

git submodule update --init --recursive
git config submodule.matrix.ignore all

cd submodules/matrix || exit
echo "$(tput setaf 4)Running rgbmatrix installation...$(tput setaf 9)"

make build-python PYTHON="$(which python3)"
sudo make install-python PYTHON="$(which python3)"
cd bindings || exit 
sudo pip3 install --force-reinstall -e python/

cd ../../../ || exit

git reset --hard
git fetch origin --prune
git pull

make

echo "Installation complete"