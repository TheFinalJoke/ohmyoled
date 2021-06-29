#!/bin/bash

if [[ `id -u` != 0]]
then
echo "Become Root"
exit 2
fi
apt update && apt upgrade -y

echo "It Could take 10-20 Mins"
apt install -y build-essential git python3-setuptools python3-pip python3-dev python3-pillow python3-numpy python3-gpiozero python3-cairosvg libatlas3-base libatlas-base-dev libraqm-dev jq pastebinit neofetch zsh dbus
apt-get install build-essential libssl-dev libffi-dev python-dev -y 

cd /usr/local/bin/ 
wget  http://python.org/ftp/python/3.8.9/Python-3.8.9.tgz 
tar -zxvf /usr/local/bin/Python-3.8.9.tgz
cd /usr/local/bin/Python-3.8.9
./configure --enable-optimizations
make altinstall
unlink /usr/bin/python3
ln -s /usr/local/bin/python3.8 /usr/bin/python3

cd /usr/local/bin/ohmyoled
pip3 install -r requirements.txt

git submodule update --init --recursive
git config submodule.matrix.ignore all

cd submodules/matrix || exit
echo "$(tput setaf 4)Running rgbmatrix installation...$(tput setaf 9)"
apt-get install build-essential libssl-dev libffi-dev python-dev

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