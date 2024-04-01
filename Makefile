# compile targets
ANDROID_TARGET=aarch64-unknown-linux-musl
WINDOWS_TARGET=x86_64-pc-windows-gnu

default: build

all: default build-windows build-aarch64

build:
	cargo build --release

# ensure that rustup has the target installed
build-windows:
	cargo build --release --target $(WINDOWS_TARGET)

# ensure that cross is installed:
# `cargo install cross`
build-aarch64:
	cross build --release --target $(ANDROID_TARGET)

# save a copy of the entire repository in `project/code`
copy:
	yes | rm -rf ./project/code
	mkdir -p ./project/code
	git clone . ./project/code

report: copy
	typst compile report/main.typ project/report.pdf

docs: copy
	cargo doc --no-deps
	cp -r target/doc project/

exe: build build-windows build-aarch64
	mkdir -p project/bin

	cp target/release/rfs_client project/bin/
	cp target/release/rfs_server project/bin/
	cp target/$(WINDOWS_TARGET)/release/rfs_client.exe project/bin/
	cp target/$(WINDOWS_TARGET)/release/rfs_server.exe project/bin/
	cp target/$(ANDROID_TARGET)/release/rfs_client project/bin/rfs_client_aarch64
	cp target/$(ANDROID_TARGET)/release/rfs_server project/bin/rfs_server_aarch64

# create the submission zip file
submit: copy report docs exe
	zip -r project.zip project

clean:
	yes | rm -rf ./project/
	rm -f ./project.zip
