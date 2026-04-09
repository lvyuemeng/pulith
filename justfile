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
    cargo deny --all-features check advisories bans sources

quality: fmt clippy test doc

verify: audit tree deny

ci: quality verify
