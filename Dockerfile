FROM ubuntu:latest

ARG TARGETARCH
ARG TARGETPLATFORM
ARG TARGETVARIANT
ARG BINARY=binary/${TARGETARCH}${TARGETVARIANT}/convis

RUN apt-get update

RUN mkdir -p  /opt/kentik
ADD $BINARY   /opt/kentik/
RUN chmod a+x /opt/kentik/convis

WORKDIR /opt/kentik/

ENTRYPOINT ["/opt/kentik/convis"]
