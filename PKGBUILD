# Maintainer: jotalea
pkgname=ms900ctl
pkgver=0.1.0
pkgrel=1
pkgdesc="Native configuration tool for the Havit MS900 gaming mouse (DPI, lighting, key bindings, macros)"
arch=('x86_64' 'aarch64')
url="https://github.com/jotalea/ms900ctl"
license=('MIT')
# hidapi is compiled from C with the cc crate; makepkg's default -flto=auto
# would turn it into GCC bitcode that rust-lld cannot link.
options=('!lto')
depends=('systemd-libs' 'libxkbcommon' 'wayland' 'libgl' 'libx11' 'libxcursor' 'libxi' 'libxrandr' 'fontconfig' 'hicolor-icon-theme')
makedepends=('cargo' 'pkgconf')
optdepends=('xorg-xinput: apply the "Mouse Parameter" panel on X11'
            'glib2: apply the "Mouse Parameter" panel on GNOME (gsettings)'
            'kconfig: apply the "Mouse Parameter" panel on KDE Plasma')
# Build straight from this directory (makepkg -si inside ms900ctl/).
# For a release tarball replace this with source=("$url/archive/v$pkgver.tar.gz").
source=()
sha256sums=()

prepare() {
    cd "$startdir"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$startdir"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR="$srcdir/target"
    cargo build --frozen --release
}

check() {
    cd "$startdir"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR="$srcdir/target"
    cargo test --frozen --release --lib
}

package() {
    cd "$startdir"
    install -Dm755 "$srcdir/target/release/ms900ctl" "$pkgdir/usr/bin/ms900ctl"
    install -Dm644 packaging/70-ms900.rules "$pkgdir/usr/lib/udev/rules.d/70-ms900.rules"
    install -Dm644 packaging/ms900ctl.desktop "$pkgdir/usr/share/applications/ms900ctl.desktop"
    install -Dm644 assets/icon.png "$pkgdir/usr/share/icons/hicolor/64x64/apps/ms900ctl.png"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 PROTOCOL.md "$pkgdir/usr/share/doc/$pkgname/PROTOCOL.md"
}
