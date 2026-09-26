set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

setup:
    pnpm install --dir ui
    cargo fetch

dev:
    cargo tauri dev

dev-full:
    cargo tauri dev --features analytics-duckdb

test:
    cargo test --workspace
    pnpm --dir ui test

lint:
    cargo fmt --check
    cargo clippy --workspace -- -D warnings
    pnpm --dir ui lint
    pnpm --dir ui typecheck

check: lint test

bench:
    cargo bench -p gr-parser-winamax
    cargo bench -p gr-stats

synth N="1000000":
    cargo run -p gr-synth --release -- --count {{N}} --out ./target/synth

perf:
    powershell -File tools/perf.ps1

snapshots:
    cargo insta review

build:
    cargo tauri build --no-bundle
