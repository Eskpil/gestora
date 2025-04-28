build-release:
	cargo build --release

install: build-release
		sudo cp ./target/release/gestora /usr/local/bin/gestora

