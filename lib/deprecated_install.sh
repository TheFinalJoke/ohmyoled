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
apt install -y build-essential git libssl-dev libffi-dev python3-openssl python-dev python3-setuptools python3-pip python3-dev python3-pillow python3-numpy python3-gpiozero python3-cairosvg libatlas3-base libatlas-base-dev libraqm-dev jq pastebinit neofetch zsh dbus libjpeg-dev zlib1g-dev
cd /usr/local/bin/ 
if [ ! -f /usr/local/bin/Python-3.8.9.tgz ]
then 
echo "Installing Python3.8.9"
wget  http://python.org/ftp/python/3.8.9/Python-3.8.9.tgz 
tar -zxvf /usr/local/bin/Python-3.8.9.tgz
cd /usr/local/bin/Python-3.8.9
./configure --enable-optimizations
make altinstall -j $(nproc)
unlink /usr/bin/python3


echo "Linking Python3.8"
ln -s /usr/local/bin/python3.8 /usr/bin/python3
fi 
rm /usr/bin/lsb_release
/usr/bin/python3 -m pip install --upgrade pip
cd $SOURCE_DIR
echo "Installing Pip3 Requirements"
python3 -m pip install ohmyoled==0.1.0 --verbose

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
############## Not Supported yet ##################
# Historical and days need
# historical=True
# days_ago=30
###################################################
EOF
echo <<EOF >> /etc/ohmyoled/ecIcons_utf8.csv
encode,Code,ForecastCode,OWMCode,Description,Represents,fontcode,font
UTF8,0,0,800,Sunny,Day Conditions Only,f00d,
UTF8,1,2,800,Mainly Sunny,Day Conditions Only,f00d,
UTF8,2,1,801,Partly Cloudy,Day Conditions Only,f002,
UTF8,3,4,801,Mostly Cloudy,Day Conditions Only,f013,
UTF8,6,6,500,Light Rain Shower,Day Conditions Only,f008,
UTF8,7,7,300,Light Rain Shower and Flurries,Day Conditions Only,f006,
UTF8,8,8,600,Light Flurries,Day Conditions Only,f00a,
UTF8,10,10,801,Cloudy,Day and Night Conditions,f013,
UTF8,11,13,500,Precipitation,Day and Night Conditions,f01c,
UTF8,12,13,500,Rain,Day and Night Conditions,f019,
UTF8,13,12,500,Rain Shower,Day and Night Conditions,f01a,
UTF8,14,14,500,Freezing Rain,Day and Night Conditions,f0b5,
UTF8,15,15,600,Rain and Snow,Day and Night Conditions,f017,
UTF8,16,16,600,Snow,Day and Night Conditions,f01b,
UTF8,16,17,600,Snow,Day and Night Conditions,f01b,
UTF8,17,17,600,Snow,Day and Night Conditions,f01b,
UTF8,18,17,600,Heavy Snow,Day and Night Conditions,f01b,
UTF8,19,9,200,Thunderstorm,Day Conditions Only,f01e,
UTF8,23,23,721,Haze,Day and Night Conditions,f0b6,
UTF8,24,24,741,Fog,Day and Night Conditions,f014,
UTF8,25,17,600,Drifting Snow,Day and Night Conditions,f064,
UTF8,26,27,600,Ice Crystals,Day and Night Conditions,f076,
UTF8,27,19,600,Hail,Day and Night Conditions,f015,
UTF8,28,28,300,Drizzle,Day and Night Conditions,f01c,
UTF8,30,30,800,Clear,Night Conditions Only,f02e,
UTF8,31,31,800,Mainly Clear,Night Conditions Only,f02e,
UTF8,32,32,801,Partly Cloudy,Night Conditions Only,f031,
UTF8,33,33,801,Mostly Cloudy,Night Conditions Only,f041,
UTF8,36,36,500,Light Rain Shower,Night Conditions Only,f02b,
UTF8,37,37,300,Light Rain Shower and Flurries,Night Conditions Only,f026,
UTF8,38,38,600,Light Flurries,Night Conditions Only,f02a,
UTF8,39,39,200,Thunderstorm,Night Conditions Only,f02d,
UTF8,40,40,600,Blowing Snow,Day and Night Conditions,f064,
UTF8,41,43,781,Funnel Cloud,Day and Night Conditions,f056,
UTF8,42,43,781,Tornado,Day and Night Conditions,f056,
UTF8,43,43,430,Windy,Day and Night Conditions,f021,
UTF8,44,44,711,Smoke,Day and Night Conditions,f062,
UTF8,45,44,761,Blowing Dust,Day and Night Conditions,f063,
UTF8,46,9,200,Thunderstorm with Hail,Day and Night Conditions,f01e,
UTF8,47,9,200,Thunderstorm with Dust Storm,Day and Night Conditions,f01e,
UTF8,48,43,781,Waterspout,Day and Night Conditions,f056,
UTF8,90,29,900,N/A,Day and Night Conditions,f07b,
UTF8,91,91,901,Humidity,Day and Night Conditions,f07a,
UTF8,92,92,902,Rising,Day and Night Conditions,f057,
UTF8,93,93,903,Falling,Day and Night Conditions,f088,
UTF8,94,94,904,Steady,Day and Night Conditions,f04d,
UTF8,1,5,800,Clear,Day Conditions Only,f00d,
UTF8,3,3,801,Cloudy Periods,Day Conditions Only,f013,
EOF
echo "Installation complete"
