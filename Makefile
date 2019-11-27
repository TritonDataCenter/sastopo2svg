#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.
#

#
# Copyright 2019 Joyent, Inc.
#
PROG =			sastopo2svg
PROTO_DIR =		proto/$(PROG)
RUST_CODE =		1
BUILD_TYPE =		release
JS_FILES =		src/sastopo2svg.js
RS_FILES =		src/main.rs src/lib.rs
JSSTYLE = 		deps/jsstyle/jsstyle
JSSTYLE_FILES =		$(JS_FILES)
JSSTYLE_FLAGS =		-f tools/jsstyle.conf
ESLINT_FILES =		$(JS_FILES)

NPM=npm
NODE=node
NPM_EXEC=$(shell which npm)
NODE_EXEC=$(shell which node)

include ./deps/eng/tools/mk/Makefile.defs

ENGBLD_REQUIRE          := $(shell git submodule update --init deps/eng)

TOP ?= $(error Unable to access eng.git submodule Makefiles.)

include ./deps/eng/tools/mk/Makefile.node_modules.defs

ifeq ($(BUILD_TYPE),release)
	CARGO_OPTS = --release
endif

.PHONY: all check
all: $(STAMP_NODE_MODULES)
	$(CARGO) build $(CARGO_OPTS)

release: all
	mkdir -p $(PROTO_DIR)/assets
	cp -f target/$(BUILD_TYPE)/$(PROG) $(PROTO_DIR)/
	cp -r images/*.png  $(PROTO_DIR)/assets/

clean::
	$(CARGO) clean

#
# Included target definitions.
#
include ./deps/eng/tools/mk/Makefile.node_modules.targ
include ./deps/eng/tools/mk/Makefile.targ

CLEAN_FILES =		$(PROTO_DIR)
