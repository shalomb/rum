build:
	@cargo build # $(release)

run: build
	@RUST_BACKTRACE=1 target/debug/rum

test:
	sqlite3 ~/.cache/rum.db -cmd '.schema' 'drop table paths;'
	target/debug/rum
	sqlite3 ~/.cache/rum.db -cmd '.schema' 'select * from paths order by score asc'

install:
	@cp target/$(target)/$(prog) ~/bin/$(prog)-$(extension)

all: build install

help:
	@echo "usage: make $(prog) [debug=1]"
