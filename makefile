all:
	cargo build
	# mv target/debug/aarch64-emulator aarch64-emulator
	-mv target/debug/aarch64-emulator aarch64-emulator

clean:
	cargo clean
	-rm -f aarch64-emulator
