set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[working-directory: 'tlab-boilerplate']
run:
    cargo run

[working-directory: 'tlab-boilerplate']
migrate:
    cargo run -- --migrate

check:
    cargo check --workspace

test:
    cargo test --workspace

sqlx-prepare:
    cargo sqlx prepare --workspace -- --all-targets
