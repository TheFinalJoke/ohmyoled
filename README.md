# ohmyoled

# Prerequistes:

You will have to install git

```
#!/bin/bash
sudo apt install git -y 
```

Clone the repo
```
git clone https://github.com/TheFinalJoke/ohmyoled.git
```

Run the Install Script
```
#!/bin/bash
chmod 755 install.sh
sudo ./install.sh
```

# Run The Script 

```
sudo systemctl start ohmyoled.service
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