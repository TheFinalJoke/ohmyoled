Steps to publish a new python packaged 
1. Install 
cd to python folder
`pip install -e .` 

2. Increament to version in setup.py
3. Build a new version
`python3 setup.py bdist_wheel`
> make sure twine is installed:
> pip install twine
4. twine upload dist/*
---------------------------------------------------
Steps to rebuild Dev and Prod Containers 

cd to container folder 

1. Increment the OHMYOLED_VERSION in the dockerfile 
2. Build Dev Docker file
`docker build -t dev_cont:latest devcontainer_build`
3. Get the Build Commit
`docker run -it dev_cont:latest /bin/hostname`
4. Commit Docker Image to Hub
`docker commit $COMMITID thefinaljoke/ohmyoleddev`
5. Push To Docker hub
`docker push thefinaljoke/ohmyoleddev`

Repeat to Prod Container

---------------------------------------------------
Change the Dockerfile inside of devcontainer

