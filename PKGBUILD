# Maintainer: Kimiblock Moe
pkgname=clashtui-git
pkgdesc="Clash Meta TUI Client"
url="https://github.com/JohanChane/clashtui"
license=("MIT")
arch=("any")
pkgver=r4.a1e9deb
pkgrel=1
makedepends=("rust" "cargo" "git")
depends=()
source=("git+https://github.com/JohanChane/clashtui.git")
md5sums=("SKIP")
provides=("clashtui")

function pkgver() {
	cd "${srcdir}/clashtui"
	printf "r%s.%s" "$(git rev-list --count HEAD)" "$(git rev-parse --short HEAD)"
}

function prepare() {
	cd "${srcdir}/clashtui"
	export RUSTUP_TOOLCHAIN=stable
	cargo fetch --target "$CARCH-unknown-linux-gnu"
}

function build() {
	cd "${srcdir}/clashtui"
	export RUSTUP_TOOLCHAIN=stable
	export CARGO_TARGET_DIR=target
	cargo build --frozen --release --all-features
}

function check() {
	cd "${srcdir}/clashtui"
	export RUSTUP_TOOLCHAIN=stable
	cargo test --frozen --all-features
}

function package() {
	install -Dm755 "${srcdir}/clashtui/target/release/clashtui" "${pkgdir}/usr/bin/clashtui"

  mkdir -p "${pkgdir}/usr/share/clashtui"
  mv ${srcdir}/clashtui/App/* "${pkgdir}/usr/share/clashtui"
}
