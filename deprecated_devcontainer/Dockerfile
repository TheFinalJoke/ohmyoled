# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.209.6/containers/python-3/.devcontainer/base.Dockerfile

FROM thefinaljoke/ohmyoleddev:arm64 AS python_layer
RUN apt install zsh -y && pip3 install pylint && pip3 install flake8 && pip3 install black
ENV DEV=1