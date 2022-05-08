#!/bin/bash

# For Testing before pushing it to pypi
# For local testing you can `pip install -e .` in the directory of src/python/
# ----------------------------------------------------------------------------
# For making sure it works properly from pypi -> docker run -it python /bin/bash
# in the container run -> apt update && apt install build-essantial 
# then python3 -m pip install ohmyoled 
# python3 and import ohmyoled, if you see failure in RgbMatrix it imported correctly!

# make sure we are in the correct directory

cd /workspaces/ohmyoled/src/python/
which twine > /dev/null
if [[ $(echo $?) != 0 ]];
then 
python3 -m pip install twine 
fi 

python3 setup.py bdist_wheel
echo "You will need access to the ohmyoled pypi, Ask thefinaljoke(nick shorter) for access"
echo "As well as a username and password"
sleep 10

twine upload dist/*