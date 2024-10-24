# To build

This project includes C sources and Rust sources.
The C sources must be built first.
[Meson](https://mesonbuild.com/index.html) is used to build the C sources.

When the project is built the first time, Meson needs to build directory
`build/meson` to be setup.

```
meson setup build/meson
```

Note that, Meson itself does not require the build directory to be called
`build/meson`. However, this project requires that. This directory name is
hardcoded for the later Rust building step (see `build.rs`). Also, `build` is
added to `.gitignore`.

Then, once the build directory is set up, you can always use the following
command to build the C part.

```
meson compile -C build/meson
```

After the C part is built, run the Rust build command, and it will
automatically use what building C sources produces

```
cargo build
```

Or `cargo test` can be directly run. This command does not only build it, but
also runs the tests to ensure that things are actually working.

`cargo test` only runs tests that do not interact with the Rime API. Rime is
not designed to be thread-safe and tampers with global objects, so tests
involving its API cannot be run concurrently, which is how `cargo test` runs
tests by default, and therefore are labeled `#[ignored]` in this project. To
run them, run

```
cargo test -- --ignored --test-threads=1
```
