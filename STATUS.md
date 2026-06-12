# Artifact Status

## Requested Badges

- Functional
- Reusable
- Available

The artifact is archived in Zenodo: 10.5281/zenodo.20670274.

## Functional Badge Justification

The artifact is documented, containerized, and exercisable. The main image
contains DepReduce binaries, build tools, examples, experiment scripts, analysis
dependencies, and the initialized data submodule.

The README gives a short path to check the tools, run a Bazel smoke test,
inspect dependency diffs, and rerun bundled-data analyses.

## Reusable Badge Justification

The packaged Rust workspace includes the optimizer, statistics tool, and shared
utilities. Bazel, Buck, and Cargo integrations are factored into support/editor
modules. Scripts, configs, examples, and bundled data support inspection and
selected reruns.

## Known Limitations

Full real-project reruns are outside the default review path because they need
external checkouts, network access, project-specific dependencies, and long
runtimes. Some external dependencies may have changed since the experiments,
so not all projects may build or run tests successfully.
We provide an optional image with a Zirgen checkout for running DepReduce
on the real-project experiments.