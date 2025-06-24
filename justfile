rootdir := ''
prefix := '/usr'
debug := '0'

name := 'cosmic-ext-applet-external-monitor-brightness'
appid := 'io.github.cosmic_utils.' + name

cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
bin-src := cargo-target-dir / if debug == '1' { 'debug' / name } else { 'release' / name }


base-dir := absolute_path(clean(rootdir / prefix))
share-dst := base-dir / 'share'

bin-dst := base-dir / 'bin' / name
desktop-dst := share-dst / 'applications' / appid + '.desktop'
metainfo-dst := share-dst / 'metainfo' / appid + '.metainfo.xml'

default: build-release


build-debug *args:
    cargo build {{args}}

build-release *args:
  cargo build --release {{args}}

install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 res/desktop_entry.desktop {{desktop-dst}}
    # install -Dm0644 res/metainfo.xml {{metainfo-dst}}

uninstall:
    rm {{bin-dst}}
    rm {{desktop-dst}}
    # rm {{metainfo-dst}}

clean:
    cargo clean

###################  Test / Format


pull: fmt prettier fix test

test:
	cargo test --workspace --all-features

fix:
	cargo clippy --workspace --all-features --fix --allow-dirty --allow-staged

fmt:
	cargo fmt --all

prettier:
	# install on Debian: sudo snap install node --classic
	# npx is the command to run npm package, node is the runtime
	npx prettier -w .



metainfo-check:
	appstreamcli validate --pedantic --explain --strict res/metainfo.xml

branch := "main"
sdk-version := "24.08"


setup:
    rm -rf android-mic
    rm -rf flatpak-builder-tools
    git clone https://github.com/teamclouday/AndroidMic.git --branch {{branch}}
    git clone https://github.com/flatpak/flatpak-builder-tools
    pip install aiohttp toml

sources-gen:
    python3 flatpak-builder-tools/cargo/flatpak-cargo-generator.py AndroidMic/RustApp/Cargo.lock -o cargo-sources.json

manifest-gen:
    ./gen_manifest.nu

install-sdk:
    flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo
    flatpak install --noninteractive --user flathub \
        org.freedesktop.Platform//{{sdk-version}} \
        org.freedesktop.Sdk//{{sdk-version}} \
        org.freedesktop.Sdk.Extension.rust-stable//{{sdk-version}} \
        org.freedesktop.Sdk.Extension.llvm18//{{sdk-version}}

uninstallf:
    flatpak uninstall {{appid}} -y || true

# deps: flatpak-builder git-lfs
build-and-install: uninstallf
    flatpak-builder \
        --force-clean \
        --verbose \
        --user --install \
        --install-deps-from=flathub \
        --repo=repo \
        flatpak-out \
        {{appid}}.json

run:
    RUST_LOG="warn,cosmic_ext_applet_external_monitor_brightness=debug" flatpak run {{appid}}

build-and-run: build-and-install run