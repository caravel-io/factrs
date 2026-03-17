install_linux_toolchain:
	rustup toolchain add stable-x86_64-unknown-linux-musl --profile minimal --force-non-host

build_macos:
	cargo build
