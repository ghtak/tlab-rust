set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[working-directory: 'tlab-boilerplate']
run:
    cargo run --bin tlab-boilerplate

[working-directory: 'tlab-boilerplate']
migrate:
    cargo run --bin tlab-boilerplate -- --migrate

[working-directory: 'tlab-boilerplate']
[env('TLAB_DATABASE__URL', 'postgres://tlab-test:tlab-test@localhost:35432/tlab-test')]
migrate-test:
    cargo run --bin tlab-boilerplate -- --migrate

[working-directory: 'tlab-boilerplate']
init-admin:
    cargo run --bin tlab-boilerplate -- --init-admin

check:
    cargo check --workspace

test:
    cargo test --workspace
