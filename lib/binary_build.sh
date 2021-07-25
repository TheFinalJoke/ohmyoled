#!/bin/bash
VERSION="0.2.0"

# Can't put full folders
echo "Building the binary"
pyinstaller --name ohmyoled --onefile ohmyoled.spec --distpath . --clean

echo "Moving ohmyoled binary and support files"
mkdir ohmyoled-$VERSION

mv -v ohmyoled ohmyoled-$VERSION/

cp -v lib/weather/ecIcons_utf8.csv ohmyoled-$VERSION

cp -v install.sh ohmyoled-$VERSION

echo "Compressing ohmyoled"
tar -zcvf ohmyoled-$VERSION.tar.gz ohmyoled-$VERSION

