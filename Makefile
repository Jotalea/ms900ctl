# ms900ctl - Havit MS900 configuration tool
#
#   make            build release binary (target/release/ms900ctl)
#   make run        build + run
#   make test       unit tests
#   make install    install binary, udev rule, desktop file, icon (PREFIX=/usr)
#   make uninstall
#   make udev       install only the udev rule and reload udev (needs root)
#   make pkg        build an Arch package with makepkg
#   make docker     build the binary inside Docker (any distro), output in dist/
#   make clean

CARGO    ?= cargo
PREFIX   ?= /usr
DESTDIR  ?=
BIN      := target/release/ms900ctl
UDEVDIR  ?= $(PREFIX)/lib/udev/rules.d

.PHONY: all build run test check install uninstall udev pkg docker clean

all: build

build:
	$(CARGO) build --release

run: build
	./$(BIN)

test:
	$(CARGO) test --lib

check:
	$(CARGO) clippy --all-targets -- -D warnings

install: build
	install -Dm755 $(BIN) $(DESTDIR)$(PREFIX)/bin/ms900ctl
	install -Dm644 packaging/70-ms900.rules $(DESTDIR)$(UDEVDIR)/70-ms900.rules
	install -Dm644 packaging/ms900ctl.desktop $(DESTDIR)$(PREFIX)/share/applications/ms900ctl.desktop
	install -Dm644 assets/icon.png $(DESTDIR)$(PREFIX)/share/icons/hicolor/64x64/apps/ms900ctl.png
	@echo "Run 'sudo udevadm control --reload && sudo udevadm trigger' (or re-plug the mouse) once."

uninstall:
	rm -f $(DESTDIR)$(PREFIX)/bin/ms900ctl \
	      $(DESTDIR)$(UDEVDIR)/70-ms900.rules \
	      $(DESTDIR)$(PREFIX)/share/applications/ms900ctl.desktop \
	      $(DESTDIR)$(PREFIX)/share/icons/hicolor/64x64/apps/ms900ctl.png

udev:
	install -Dm644 packaging/70-ms900.rules $(DESTDIR)$(UDEVDIR)/70-ms900.rules
	udevadm control --reload
	udevadm trigger --subsystem-match=hidraw

pkg:
	makepkg -sf

docker:
	docker build -t ms900ctl-build .
	mkdir -p dist
	docker run --rm -v "$(CURDIR)/dist:/out" ms900ctl-build

clean:
	$(CARGO) clean
	rm -rf dist pkg src/*.tar.gz
