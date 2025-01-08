rootdir := ''
prefix := '/usr'
debug := '0'

export NAME := 'cosmic-ext-applet-external-monitor-brightness'
export APPID := 'io.github.maciekk64.CosmicExtAppletExternalMonitorBrightness'



cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
bin-src := cargo-target-dir / if debug == '1' { 'debug' / NAME } else { 'release' / NAME }


base-dir := absolute_path(clean(rootdir / prefix))

bin-dst := base-dir / 'bin' / NAME
desktop-dst := base-dir / 'share' / 'applications' / APPID + '.desktop'

default: build-release


build-debug *args:
    cargo build {{args}}

build-release *args:
  cargo build --release {{args}}

install:
    install -Dm0755 {{bin-src}} {{bin-dst}}
    install -Dm0644 res/desktop_entry.desktop {{desktop-dst}}

uninstall:
    rm {{bin-dst}}

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
