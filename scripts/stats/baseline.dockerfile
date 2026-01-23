FROM depreduce:latest

ARG REPO="copybara"
ARG COMMIT="caf9542f3be81bb3f50398194337f969d83d3131"
ARG EXTRA_ARGS=""
ARG REVERT_COMMIT="38598c5bcd180cf2e887eccc19b986eebb87f79d"
ARG PRERUN="echo \"no prerun\""
ARG POSTRUN="echo \"no postrun\""

WORKDIR /app/${REPO}

RUN git checkout ${COMMIT}
RUN if [ -n "${REVERT_COMMIT}" ]; then \
    git revert ${REVERT_COMMIT}; \
    fi
RUN git submodule update

RUN ${PRERUN}
RUN bazel build --spawn_strategy=local \
    ${EXTRA_ARGS} \
    //...
RUN ${POSTRUN}

RUN bazel shutdown