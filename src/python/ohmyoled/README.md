# ohmyoled

# Prerequistes:

Download the binary and support files
Go to releases and download the most recent tag
```
wget https://github.com/TheFinalJoke/ohmyoled/releases/download/$tag/ohmyoled-$tag.tar.gz
```
Uncompress the File
```
tar -zxvf ohmyoled-$tag.tar.gz
```
Run the Install Script
```
#!/bin/bash
cd ohmyoled-$tag
chmod 755 install.sh
sudo ./install.sh
```

# Run The Script 

```
sudo systemctl enable --now ohmyoled.service
```

# Configuration 
This is the Basic of Linux Configuration Files located /etc/ohmyoled/ohmyoled.conf

Run Different Modules Set the Modules Run Attribute to True

*Note: Most needs to have API Tokens assoicated to The Module 
* You will also need to Reload The service after any changes

```
[weather]
Run=True
```

You Can also change different Configurations like weather 
You can add different zipcode/city 

# To Help Write Our and use tests
```
pip install -e .[dev]
```

# Development
Open vscode and make sure you have docker on your raspberry pi
Then connect to remote raspberry pi or run vscode on raspberry pi
Choose to reopen in container and you will have the environment to develop

# Soon To come: Docker Container
# Known Problems

// If your pi is not booting, you will have to change the kernel bootloader. With Raspbain OS the bootloader is borked with HAT Software
https://learn.adafruit.com/adafruit-1-3-color-tft-bonnet-for-raspberry-pi/kernel-module-troubleshooting
