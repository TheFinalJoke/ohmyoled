#!/bin/bash
SOURCE_DIR=$(pwd)

if [[ `id -u` != 0 ]]
then
echo "Are you root?"
exit 2
fi
apt update && apt upgrade -y && apt install -y fonts-noto-mono git
if [ 0 == $(echo $?) ]
then
echo "Error Occured While installing and updatings fonts"
exit 1
fi 

mv -v $SOURCE_DIR/fonts/* /usr/share/fonts/
mv -v $SOURCE_DIR/ohmyoled /usr/local/bin/
if [ -f /usr/lib/systemd/system/ohmyoled.service ]
then
echo "ohmyoled service exists"
else
echo "Creating Systemd File for ohmyoled"
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
fi
 
ln -s /usr/lib/systemd/system/ohmyoled.service /etc/systemd/system/multi-user.target.wants/
systemctl daemon-reload

mkdir -p /etc/ohmyoled/ && mkdir -p /var/log/ohmyoled/
if [ 0 == $(echo $?) ]
then
echo "Error Occured creating ohmyoled dirs"
exit 1
fi 

echo "Created OhMyOled Config file to /etc/ohmyoled"
if [ -f /etc/ohmyoled/ohmyoled.conf ] 
then
echo "/etc/ohmyoled/ohmyoled.conf already exists"
else
mv -r $SOURCE_DIR/ohmyoled.conf /etc/ohmyoled/
fi 
