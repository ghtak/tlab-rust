set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[working-directory: 'tlab-boilerplate']
run:
    cargo run --bin tlab-boilerplate

[working-directory: 'tlab-boilerplate']
migrate:
    cargo run --bin migrate

[working-directory: 'tlab-boilerplate']
init-admin:
    cargo run --bin tlab-boilerplate -- --init-admin

check:
    cargo check --workspace

test:
    cargo test --workspace

sqlx-prepare:
    cargo sqlx prepare --workspace -- --all-targets
