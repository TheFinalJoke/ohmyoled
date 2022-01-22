#!/bin/bash

docker build container_build -t ohmyoled

if [[ echo $? != 0 ]];
then 
echo "I'm not sure if know this.... But something failed..."
exit 1
fi 

echo "Succesfully built docker image"

echo "Login to docker hub"
docker login

echo "Push the commit to docker hub"

echo "Example: docker commit 0274fdd87f06 thefinaljoke/ohmyoled:dev"
echo "Example Push: docker push thefinaljoke/ohmyoled:1.3.4"