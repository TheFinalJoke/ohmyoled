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
python3 -m pip install -r requirements.txt --no-cache-dir --verbose --ignore-installed

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

echo "Creating Systemd File"
cat <<EOF >> /usr/lib/systemd/system/ohmyoled.service
[Unit]
Description=OhMyOLED Service

[Service]
ExecStart=/usr/bin/python3 $SOURCE_DIR/main.py 
User=root
WorkingDirectory=$SOURCE_DIR

[Install]
WantedBy=multi-user.target
EOF
ln -s /usr/lib/systemd/system/ohmyoled.service /etc/systemd/system/multi-user.target.wants/
systemctl daemon-reload

mkdir -p /etc/ohmyoled/

echo "Created OhMyOled Config file to /etc/ohmyoled"
cat <<EOF >> /etc/ohmyoled/ohmyoled.conf
[basic]
# Log Level for Python
# Critcal = 50
# Error = 40
# Warning = 30
# Info = 20
# Debug = 10
loglevel = 10
# Tokens will have to be Supported used with Approriate Modules
#### Not Supported Yet #####
# sport_token = 'SPORT_TOKEN'
##############################
# Stock Token based in FINN TOKEN https://finnhub.io/docs/api/introduction
# stock_api_token = YOUR_FINN_TOKEN

# Open Weather Token in https://openweathermap.org/
# open_weather_token = open_weather_token 

[matrix]
chain_length = 1
parallel = 1
# Brightness from 0-100
brightness = 100
oled_slowdown = 3

[weather]
Run=False
# Build the city as Dallas, US, For more accurate data, use zipcode
#city = 
zipcode =  
# Format
# The Units, The options are standard, metric, imperial
format = imperial

[stock]
Run=False

# symbol=

# Runs Quote data for stocks
# quote=True

# Historical and days need
# historical=True
# days_ago=30
EOF
echo "Installation complete"
