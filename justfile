set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

run config="tlab-cli/config.yml":
    cargo run -p tlab-cli -- --config "{{config}}"

sqlx-prepare:
    cargo sqlx prepare --workspace -- --all-targets