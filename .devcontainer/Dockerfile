# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.209.6/containers/python-3/.devcontainer/base.Dockerfile

FROM thefinaljoke/ohmyoleddev:latest AS python_layer
RUN apt install zsh -y \
&& pip3 install pylint \
&& pip3 install flake8 \
&& pip3 install black \
&& pip3 install grpcio \
&& pip3 install grpcio-tools
ENV DEV=1
RUN echo "PROMPT=\"%n@%m:%~ [code: %?] \"" >> /root/.zshrc