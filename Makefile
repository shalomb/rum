build:
	@cargo build # $(release)

run:
	@RUST_BACKTRACE=1 target/debug/rum

install:
	@cp target/$(target)/$(prog) ~/bin/$(prog)-$(extension)

all: build install

help:
	@echo "usage: make $(prog) [debug=1]"
