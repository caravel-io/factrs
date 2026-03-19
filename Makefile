install_linux_toolchain:
	rustup toolchain add stable-x86_64-unknown-linux-musl --profile minimal --force-non-host

build_macos:
	cargo build

build_linux:
	cross build --target aarch64-unknown-linux-musl

vagrant_start: build_linux
	vagrant up --provider=utm
	vagrant scp target/aarch64-unknown-linux-musl/debug/factrs :~/factrs
	vagrant ssh -c "bash -c '~/factrs | jq'"

vagrant_refresh: build_linux
	vagrant scp target/aarch64-unknown-linux-musl/debug/factrs :~/factrs
	vagrant ssh -c "bash -c '~/factrs | jq'"
