rootdir := ''
prefix := '/usr'
debug := '0'

name := 'cosmic-ext-applet-external-monitor-brightness'
export APPID := 'io.github.maciekk64.CosmicExtAppletExternalMonitorBrightness'



cargo-target-dir := env('CARGO_TARGET_DIR', 'target')
bin-src := cargo-target-dir / if debug == '1' { 'debug' / NAME } else { 'release' / NAME }


base-dir := absolute_path(clean(rootdir / prefix))

bin-dst := base-dir / 'bin' / name
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