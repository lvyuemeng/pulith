set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
    just --list

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all --check

check:
    cargo check --workspace --all-features

clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

test:
    cargo test --workspace --all-features

doc:
    $env:RUSTDOCFLAGS='-D warnings'; cargo doc --workspace --all-features --no-deps

audit:
    cargo audit

tree:
    cargo tree --workspace --all-features -d

deny:
    cargo deny check --all-features advisories bans sources

verify: fmt-check clippy test doc

ci: verify audit tree deny
