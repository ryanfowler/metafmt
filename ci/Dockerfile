ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE
ENV GO_VERSION=1.24.0
RUN apt-get update && apt-get -y install wget && \
    wget -qO go${GO_VERSION}.linux-amd64.tar.gz https://dl.google.com/go/go${GO_VERSION}.linux-amd64.tar.gz && \
    tar -C /usr/local -xzf go${GO_VERSION}.linux-amd64.tar.gz
ENV PATH=$PATH:/usr/local/go/bin

# Workaround for the go tool not having permission to write elsewhere.
ENV GOPATH=/tmp/gopath
ENV GOCACHE=/tmp/gocache
