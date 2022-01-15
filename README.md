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