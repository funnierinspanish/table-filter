PROGRAM ?= table-filter
NEWNAME ?= tf
DEST    ?= $(HOME)/.local/bin
BUILD_FLAGS ?= --release

PROFILE := $(if $(findstring --release,$(BUILD_FLAGS)),release,debug)
BIN     := ./target/$(PROFILE)/$(PROGRAM)

.PHONY: all build install reinstall uninstall run watch view pathcheck

all: build

build:
	cargo build $(BUILD_FLAGS)
	@test -x "$(BIN)" || (echo "Error: couldn't find built binary at $(BIN). Check PROGRAM name." && exit 1)

install: build
	mkdir -p "$(DEST)"
	cp "$(BIN)" "$(DEST)/$(NEWNAME)"
	chmod +x "$(DEST)/$(NEWNAME)"
	@echo "✅ Installed: $(DEST)/$(NEWNAME)"
	@$(MAKE) -s pathcheck

reinstall: uninstall install

uninstall:
	@rm -f "$(DEST)/$(NEWNAME)" && echo "🗑️  Removed: $(DEST)/$(NEWNAME)" || true

run: build
	"$(BIN)"

# Rebuild on changes and re-copy to DEST/NEWNAME using cargo-watch (optional)
# Requires: cargo install cargo-watch
watch:
	@command -v cargo-watch >/dev/null 2>&1 || { echo "Install cargo-watch: cargo install cargo-watch"; exit 1; }
	cargo watch -x "build $(BUILD_FLAGS)" -s 'mkdir -p "$(DEST)"; cp "$(BIN)" "$(DEST)/$(NEWNAME)"; echo "🔁 Updated $(DEST)/$(NEWNAME)"'

# Quickly watch the installed program's output every second
view:
	@command -v watch >/dev/null 2>&1 || { echo "Install watch (procps-ng)."; exit 1; }
	watch -n 1 "$(DEST)/$(NEWNAME)"

# Warn if ~/.local/bin isn’t on PATH
pathcheck:
	@echo "$$PATH" | tr ':' '\n' | grep -qx "$(DEST)" || \
	  echo "⚠️  Note: $(DEST) is not on your PATH. Add this to your shell rc:\n  export PATH=\"$(DEST):$$PATH\""
