# Build environment for the prototype: its own target and build dirs, no inherited lld flag.
export CARGO_TARGET_DIR=$HOME/.cache/attn-browser-proto/target
export CARGO_BUILD_BUILD_DIR=$HOME/.cache/attn-browser-proto/target/build.noindex
export RUSTFLAGS=" "
export CEF_PATH=$HOME/.cache/attn-browser-proto/cef
