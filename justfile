# Fail on early and on unset variables in non-shebang recipes
set shell := ["bash", "-euo", "pipefail", "-c"]
# Allow usage of bash methods to handle multiple arguments and work around quoting issues
set positional-arguments

rust_nightly_version := `cat rust-toolchain-nightly`

@default: fmt lint test

fmt:
    cargo '+{{rust_nightly_version}}' fmt --all

bumpdeps:
    cargo install cargo-edit
    cargo upgrade

lint: lint-code

lint-code strict="":
    cargo '+{{rust_nightly_version}}' fmt -- --check
    cargo clippy \
        --workspace \
        --tests \
        --benches \
        --all-targets \
        --all-features \
        --quiet \
        -- {{ if strict != "" { "-D warnings" } else { "" } }}
    cargo doc --all --no-deps --document-private-items --quiet

test: test-unit

test-unit:
    cargo test --workspace --bins --examples --tests
