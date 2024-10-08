# This is meant for Running a basic container during development
ARG VARIANT="3.9-bullseye"
FROM python:${VARIANT} AS python_layer

# [Choice] Node.js version: none, lts/*, 16, 14, 12, 10
ARG NODE_VERSION="none"
RUN if [ "${NODE_VERSION}" != "none" ]; then su vscode -c "umask 0002 && . /usr/local/share/nvm/nvm.sh && nvm install ${NODE_VERSION} 2>&1"; fi
RUN apt update && apt install fonts-noto-mono -y && apt install vim python3-dev cython3 -y 

RUN /usr/local/bin/python3 -m pip install --upgrade pip
RUN python3 -m pip install Cython
RUN git clone https://github.com/hzeller/rpi-rgb-led-matrix.git
RUN cd rpi-rgb-led-matrix && make build-python PYTHON="$(which python3)" && make install-python PYTHON="$(which python3)" && cd bindings && python3 -m pip install -e python/ -I

RUN export CFLAGS=-fcommon && python3 -m pip install Pyinstaller RPi.GPIO
RUN export CFLAGS=-fcommon && python3 -m pip install ohmyoled==2.2.8

FROM python_layer AS rust_layer
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc
ENV PATH="/root/.cargo/bin:${PATH}"
FROM rust_layer
RUN git clone https://github.com/TheFinalJoke/ohmyoled.git \
    && mkdir -p /var/log/ohmyoled && mkdir -p /etc/ohmyoled \
    && cp -Rv /ohmyoled/fonts/* /usr/share/fonts/
WORKDIR /ohmyoled/
RUN /root/.cargo/bin/cargo build --release
ENV RUST_BACKTRACE=full
ENTRYPOINT ["target/release/ohmyoled"]
