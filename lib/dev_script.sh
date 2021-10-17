#!/bin/bash
SOURCE_DIR=$(pwd)
PYTHONTAG='3.9.7'

if [[ `id -u` != 0 ]]
then
echo "Become Root"
exit 2
fi
apt update && apt upgrade -y

echo "Installing Build Essentials and Dependencies"
sleep 5
apt install -y build-essential git libssl-dev libffi-dev python3-openssl python-dev python3-setuptools python3-pip python3-dev \
python3-pillow python3-numpy python3-gpiozero python3-cairosvg libatlas3-base libatlas-base-dev \
libraqm-dev jq pastebinit neofetch zsh dbus libjpeg-dev zlib1g-dev libxml2-dev libxslt-dev python-dev libbz2-dev liblzma-dev

cd /usr/local/bin/ 
if [ ! -f /usr/local/bin/Python-$PYTHONTAG.tgz ]
then 
echo "Installing Python$PYTHONTAG"
wget  http://python.org/ftp/python//Python-$PYTHONTAG.tgz 
tar -zxvf /usr/local/bin/Python-$PYTHONTAG.tgz
cd /usr/local/bin/Python-$PYTHONTAG
./configure --enable-optimizations
make altinstall -j $(nproc)
unlink /usr/bin/python3


echo "Linking Python3.9"
ln -s /usr/local/bin/python3.9 /usr/bin/python3
fi 
rm /usr/bin/lsb_release
/usr/bin/python3 -m pip install --upgrade pip
cd $SOURCE_DIR
echo "Installing Pip3 Requirements"
python3 -m pip install numpy
python3 -m pip install pandas
python3 setup.py install

git submodule update --init --recursive
git config submodule.rgbmatrix.ignore all

cd submodules/rgbmatrix || exit
echo "$(tput setaf 4)Running rgbmatrix installation...$(tput setaf 9)"

make build-python PYTHON="$(which python3)"
make install-python PYTHON="$(which python3)"
cd bindings || exit 
python3 -m pip install -e python/ -I

cd ../../../ || exit

git reset --hard
git fetch origin --prune
git pull

make

echo "Finished Development"