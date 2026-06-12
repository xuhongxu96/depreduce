# Artifact Requirements

## Packaged Platform

- Architecture: Linux x86_64.
- Main archive: `depreduce-artifact-issta2026.tar.gz`.
- Main image tag: `depreduce-artifact:issta2026`.
- Optional Zirgen archive: `depreduce-artifact-zirgen-issta2026.tar.gz`.
- Optional Zirgen tag: `depreduce-artifact-zirgen:issta2026`.
- Base image: `rust:1.91.0-bookworm`.

## Reviewer Requirements

Reviewers need Docker or a compatible runtime:

```sh
docker load -i depreduce-artifact-issta2026.tar.gz
docker run --rm -it depreduce-artifact:issta2026
```

Recommended for the default path:

- CPU: 2 or more x86_64 cores.
- Memory: 8 GB RAM.
- Disk: at least 30 GB free.
- Network: basic; Bazel cache misses can still trigger downloads.

Optional Zirgen path:

- Disk: at least 100 GB free for the loaded optional image.
- Network: basic; Bazel cache misses can still trigger downloads.

Full real-project reruns need more CPU, memory, disk, network access, and time.
Some external dependencies may no longer be available,
so some projects may fail to build or run tests.

## Software Included in the Image

- Rust 1.91.0 and Cargo.
- DepReduce release binaries: `depreduce` and `depstat`.
- Bazel 8.3.1 via Bazelisk.
- Buck2, installed from the Linux release binary used by the project CI.
- OpenJDK 17.
- C/C++ build tools.
- Python 3 with `numpy`, `scipy`, `pandas`, `matplotlib`, and `seaborn`.
- `strace`, `git`, `curl`, `wget`, `unzip`, and `zstd`.

## License

The artifact source code is distributed under the terms in [LICENSE](LICENSE)
and [LICENSE.md](LICENSE.md).
