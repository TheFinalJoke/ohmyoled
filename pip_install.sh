#!/bin/bash
SOURCE_DIR=$(pwd)

if [[ `id -u` != 0 ]]
then
echo "Become Root"
exit 2
fi
apt update && apt upgrade -y

echo "It Could take 10-20 Mins"
echo "On RASPBERRY PI ZERO It Could take longer than 20 mins"
echo "Installing Build Essentials"
sleep 5
apt install -y build-essential git python3-setuptools python3-pip python3-dev python3-pillow python3-numpy python3-gpiozero python3-cairosvg libatlas3-base libatlas-base-dev libraqm-dev jq pastebinit neofetch zsh dbus libjpeg-dev zlib1g-dev

cd /usr/local/bin/ 
if [ ! -f /usr/local/bin/Python-3.8.9.tgz ]
then 
echo "Installing Python3.8.9"
wget  http://python.org/ftp/python/3.8.9/Python-3.8.9.tgz 
tar -zxvf /usr/local/bin/Python-3.8.9.tgz
cd /usr/local/bin/Python-3.8.9
./configure --enable-optimizations
make altinstall
unlink /usr/bin/python3

echo "Linking Python3.8"
ln -s /usr/local/bin/python3.8 /usr/bin/python3
fi 
rm /usr/bin/lsb_release
/usr/bin/python3 -m pip install --upgrade pip
cd $SOURCE_DIR
echo "Installing Pip3 Requirements"

