# Contributing to `source-map-mappings`

Hi! We'd love to have your contributions! If you want help or mentorship, reach
out to us in a GitHub issue, or ping `fitzgen` in [#rust on irc.mozilla.org](irc://irc.mozilla.org#rust)
and introduce yourself.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->


- [Code of Conduct](#code-of-conduct)
- [Building](#building)
- [Testing](#testing)
- [Automatic code formatting](#automatic-code-formatting)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Code of Conduct

We abide by the [Rust Code of Conduct][coc] and ask that you do as well.

[coc]: https://www.rust-lang.org/en-US/conduct.html

## Building

To build for the host target (for use with testing):

```
$ cargo build
```

To build for WebAssembly, ensure that you have the `wasm32-unknown-unknown` target:

```
$ rustup update
$ rustup target add wasm32-unknown-unknown --toolchain nightly
```

Then, cross compile to `wasm` via the C API crate:

```
$ cd source-map-mappings-wasm-api/
$ cargo build --release --target wasm32-unknown-unknown
```

The resulting `wasm` file will be located at
`source-map-mappings-wasm-api/target/wasm32-unknown-unknown/release/source_map_mappings.wasm`.

To minimize its size, run `wasm-gc` on it:

```
$ cargo install wasm-gc # If you don't already have it.
$ wasm-gc source-map-mappings-wasm-api/target/wasm32-unknown-unknown/release/source_map_mappings.wasm \\
    source-map-mappings-wasm-api/target/wasm32-unknown-unknown/release/source_map_mappings.small.wasm
```

## Testing

The tests require `cargo-readme` to be installed:

```
$ cargo install cargo-readme
```

To run all the tests:

```
$ cargo test
```

## Automatic code formatting

We use [`rustfmt`](https://github.com/rust-lang-nursery/rustfmt) to enforce a
consistent code style across the whole code base.

You can install the latest version of `rustfmt` with this command:

```
$ rustup update nightly
$ cargo install -f rustfmt-nightly
```

Ensure that `~/.cargo/bin` is on your path.

Once that is taken care of, you can (re)format all code by running this command:

```
$ cargo fmt
```
