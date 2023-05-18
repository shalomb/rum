#!/usr/bin/make -f console

MAKEFILE          := $(realpath $(lastword $(MAKEFILE_LIST)))
MAKE              := make
MAKEFLAGS         += --no-print-directory
MAKEFLAGS         += --warn-undefined-variables

.ONESHELL:
SHELL             := /bin/bash

# https://dustinrue.com/2021/08/parameters-in-a-makefile/
# setup arguments
RUN_ARGS          := $(wordlist 2,$(words $(MAKECMDGOALS)),$(MAKECMDGOALS))
# ...and turn them into do-nothing targets
$(eval $(RUN_ARGS):;@:)

build:
	@cargo build # $(release)

run: build
	@RUST_BACKTRACE=1 RUST_LOG=debug,main=debug target/debug/rum $(RUN_ARGS)

test:
	sqlite3 ~/.cache/rum.db -cmd '.schema' 'drop table paths;'
	target/debug/rum
	sqlite3 ~/.cache/rum.db -cmd '.schema' 'select * from paths order by score asc'

install:
	@cp target/$(target)/$(prog) ~/bin/$(prog)-$(extension)

all: build install

help:
	@echo "usage: make $(prog) [debug=1]"
