# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.209.6/containers/python-3/.devcontainer/base.Dockerfile

# [Choice] Python version (use -bullseye variants on local arm64/Apple Silicon): 3, 3.10, 3.9, 3.8, 3.7, 3.6, 3-bullseye, 3.10-bullseye, 3.9-bullseye, 3.8-bullseye, 3.7-bullseye, 3.6-bullseye, 3-buster, 3.10-buster, 3.9-buster, 3.8-buster, 3.7-buster, 3.6-buster
ARG VARIANT="latest"
FROM thefinaljoke/ohmyoleddev:${VARIANT} AS PYTHON_LAYER
RUN apt install zsh -y && pip3 install pylint && pip3 install flake8
ENV DEV=1
RUN echo "PROMPT=\"%n@%m:%~ [code: %?] \"" >> /root/.zshrc