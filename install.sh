#!/bin/bash
SOURCE_DIR=$(pwd)

if [[ `id -u` != 0 ]]
then
echo "Are you root?"
exit 2
fi
apt update && apt upgrade -y
apt install -y fonts-noto-mono

mv -v $SOURCE_DIR/fonts/* /usr/share/fonts/
mv -v $SOURCE_DIR/ohmyoled /usr/local/bin/
mv -v $SOURCE_DIR/ecIcons_utf8.csv /etc/ohmyoled/

echo "Creating Systemd File"
cat <<EOF >> /usr/lib/systemd/system/ohmyoled.service
[Unit]
Description=OhMyOLED Service

[Service]
ExecStart=/usr/local/bin/ohmyoled 
User=root
WorkingDirectory=/usr/local/bin/

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