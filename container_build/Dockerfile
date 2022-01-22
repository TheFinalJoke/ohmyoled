ARG VARIANT="3.8-bullseye"
FROM python:${VARIANT} AS PYTHON_LAYER

# [Choice] Node.js version: none, lts/*, 16, 14, 12, 10
ARG NODE_VERSION="none"
RUN if [ "${NODE_VERSION}" != "none" ]; then su vscode -c "umask 0002 && . /usr/local/share/nvm/nvm.sh && nvm install ${NODE_VERSION} 2>&1"; fi
RUN apt update && apt install fonts-noto-mono -y

ARG OHMYOLED_VERSION=1.3.4
ARG RBGMATRIX_GIT_URL=https://github.com/hzeller/rpi-rgb-led-matrix.git
RUN /usr/local/bin/python3 -m pip install --upgrade pip

RUN git clone ${RBGMATRIX_GIT_URL}
RUN cd rpi-rgb-led-matrix && make build-python PYTHON="$(which python3)" && make install-python PYTHON="$(which python3)" && cd bindings && python3 -m pip install -e python/ -I

RUN export CFLAGS=-fcommon && python3 -m pip install Pyinstaller RPi.GPIO
RUN export CFLAGS=-fcommon && python3 -m pip install ohmyoled==${OHMYOLED_VERSION}

FROM PYTHON_LAYER as RUST_LAYER
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

FROM RUST_LAYER
RUN git clone https://github.com/TheFinalJoke/ohmyoled.git \
    && mkdir -p /var/log/ohmyoled && mkdir -p /etc/ohmyoled \ 
    && cp /ohmyoled/lib/config/ohmyoled.conf /etc/ohmyoled/ \
    && cp -Rv /ohmyoled/fonts/* /usr/share/fonts/

CMD [ "/bin/bash"]
