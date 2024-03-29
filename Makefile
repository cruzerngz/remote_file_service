
ANDROID_TARGET=aarch64-linux-android # not used
WINDOWS_TARGET=x86_64-pc-windows-gnu

default: build

all: default build-windows

build:
	cargo build --release

build-windows:
	cargo build --release --target $(WINDOWS_TARGET)

# save a copy of the entire repository in `project/code`
copy:
	mkdir -p ./project/code
	yes | rm -r ./project/code
	git clone . ./project/code

report: copy
	typst compile report/main.typ project/report.pdf

docs: copy
	cargo doc
	cp -r target/doc project/

exe: build build-windows
	mkdir -p project/bin

	cp target/release/rfs_client project/bin/
	cp target/release/rfs_server project/bin/
	cp target/$(WINDOWS_TARGET)/release/rfs_client.exe project/bin/
	cp target/$(WINDOWS_TARGET)/release/rfs_server.exe project/bin/

# create the submission zip file
submit: copy report docs exe
	zip -r project.zip project

clean:
	yes | rm -r ./project/
	rm ./project.zip
