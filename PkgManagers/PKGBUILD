# Maintainer: Kimiblock Moe
# Maintainer: JohanChane

pkgname=clashtui-git
pkgdesc="Mihomo (Clash.Meta) TUI Client"
url="https://github.com/JohanChane/clashtui"
license=("MIT")
arch=("any")
pkgver=0.2.0.r8.gd6e96fb0
pkgrel=1
makedepends=("rust" "cargo" "git")
depends=("gcc-libs" "glibc")
source=("git+https://github.com/JohanChane/clashtui.git#branch=main")
md5sums=("SKIP")
provides=("clashtui")
conflicts=("clashtui")
options=(!lto)

function pkgver() {
	cd "${srcdir}/clashtui/clashtui"
	git describe --long --tags --abbrev=8 | sed 's/^v//;s/\([^-]*-g\)/r\1/;s/-/./g'
}

function prepare() {
	cd "${srcdir}/clashtui/clashtui"
	export RUSTUP_TOOLCHAIN=stable
	cargo fetch --target "$CARCH-unknown-linux-gnu"
}

function build() {
	cd "${srcdir}/clashtui/clashtui"
	export RUSTUP_TOOLCHAIN=stable
	export CARGO_TARGET_DIR=target
	cargo build --release --frozen --all-features --locked
}

function check() {
	cd "${srcdir}/clashtui/clashtui"
	export RUSTUP_TOOLCHAIN=stable
	cargo test --release --frozen --all-features --locked
}

function package() {
	install -Dm755 "${srcdir}/clashtui/clashtui/target/release/clashtui" "${pkgdir}/usr/bin/clashtui"
	install -Dm644 "${srcdir}/clashtui/LICENSE" "${pkgdir}/usr/share/licenses/${pkgname}/LICENSE"
}
