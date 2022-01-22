#!/bin/bash
VERSION="2.0.0"

# Can't put full folders
echo "Building the binary"
pyinstaller --name ohmyoled --onefile ohmyoled.spec --distpath . --clean

echo "Moving ohmyoled binary and support files"
mkdir ohmyoled-$VERSION

mv -v ohmyoled ohmyoled-$VERSION/

cp -v lib/config/ohmyoled.conf ohmyoled-$VERSION

cp -v install.sh ohmyoled-$VERSION

cp -vr fonts/ ohmyoled-$VERSION

echo "Compressing ohmyoled"
tar -zcvf ohmyoled-$VERSION.tar.gz ohmyoled-$VERSION

