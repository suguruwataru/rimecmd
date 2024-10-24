# To build

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
