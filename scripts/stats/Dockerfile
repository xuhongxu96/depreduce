FROM ubuntu:latest

RUN apt-get update && apt-get install -y \
    openjdk-11-jdk \
    wget \
    unzip \
    git \
    curl \
    zip \
    gcc \
    make \
    python3 \
    python3-pip \
    vim \
    sudo \
    libxdamage1 \
    libxfixes3 \
    libxkbcommon0 \
    clang \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN wget https://github.com/bazelbuild/bazelisk/releases/download/v1.28.1/bazelisk-linux-amd64
RUN mv bazelisk-linux-amd64 /usr/local/bin/bazel
RUN chmod 777 /usr/local/bin/bazel
RUN chmod -R 777 /app

ARG USERNAME=app
ARG USER_UID=2000
ARG USER_GID=$USER_UID

# Create the user
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME

USER $USERNAME

# python3 scripts/stats/test_incre_build.py --config scripts/stats/configs/angular.json

RUN git clone https://github.com/angular/angular.git /app/angular
RUN git clone https://github.com/buildfarm/buildfarm.git /app/buildfarm
RUN git clone https://github.com/google/copybara.git /app/copybara && echo "8.5.0" > /app/copybara/.bazelversion
RUN git clone https://gerrit.googlesource.com/gerrit /app/gerrit && git -C /app/gerrit submodule update --init --recursive
RUN git clone https://github.com/grpc/grpc-java.git /app/grpc-java
RUN git clone https://github.com/JetBrains/hirschgarten.git /app/hirschgarten
RUN git clone https://github.com/apache/rocketmq.git /app/rocketmq
RUN git clone https://github.com/typedb/typedb.git /app/typedb

# Cannot build successfully
# RUN git clone https://github.com/trpc-group/trpc-cpp.git /app/trpc-cpp && echo "7.7.0" > /app/trpc-cpp/.bazelversion

COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /bin/

COPY ./scripts /app/scripts
COPY ./data/experiment /app/data/experiment

RUN sudo chmod -R 777 /app/scripts /app/data

RUN uv venv --python 3.12
RUN /bin/bash -c "source ./.venv/bin/activate && uv pip install pydantic gitpython"
RUN git config --global user.name "Hongxu Xu"
RUN git config --global user.email "h445xu@uwaterloo.ca"

ENV PYTHONPATH="/app"
ENTRYPOINT ["/bin/bash"]
