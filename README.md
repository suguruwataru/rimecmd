# To build

This project includes C sources and Rust sources.
The C sources must be built first.
[Meson](https://mesonbuild.com/index.html) is used to build the C sources.

When the project is built the first time, Meson needs to build directory
`meson_build` to be setup.

```
meson setup meson_build
```

Not that, Meson itself does not require the build directory to be called
`meson_build`. However, this is required for this project. This directory
name is hardcoded in the later Rust building step (see `build.rs`). Also,
`.gitignore` uses it.

Then, once the build directory is set up, you can always use the following
command to build the C part.

```
meson -C meson_build compile
```

After the C part is built, run the Rust build command, and it will
automatically use what building C sources produces

```
cargo build
```

Or `cargo test -- --test-threads=1` can be directly run. This command does not
only build it, but also runs the tests to ensure that things are actually
working. Though tests might pass when running it without parallelization (i.e.
without `--test-threads=1`, and they most likely will not pass), Rime is not
designed to be thread-safe and tampers with global objects, so the tests must
be run in a single thread.
