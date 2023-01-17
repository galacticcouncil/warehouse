FROM ubuntu:18.04

WORKDIR /home/

COPY . .

RUN bash ./setup.sh

RUN bash ./build-deps.sh

ENV PATH="/root/.cargo/bin:$PATH"