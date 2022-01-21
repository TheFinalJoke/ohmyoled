#!/bin/bash

docker build container_build -t ohmyoled_container

if [[ echo $? != 0 ]];
then 
echo "I'm not sure if know this.... But something failed..."
exit 1
fi 

echo "Succesfully built docker image"