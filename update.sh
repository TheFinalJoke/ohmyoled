SOURCE_DIR=$(pwd)

if [[ `id -u` != 0 ]]
then
echo "Are you root?"
exit 2
fi
CURRENT_TAG=$(git describe --abbrev=0 --tags)
NEW_TAG=$(git describe --tags `git rev-list --tags --max-count=1`)
if [[ CURRENT_TAG != NEW_TAG ]]
then 
