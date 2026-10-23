#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != Darwin ]]; then
    echo "This example bundle requires macOS." >&2
    exit 1
fi

repository_root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
cd "$repository_root"
cargo build -p gpui --example window_lifecycle --locked \
    --features gpui_platform/runtime_shaders \
    --config profile.dev.debug=0 --config profile.dev.incremental=false

target_dir="${CARGO_TARGET_DIR:-$repository_root/target}"
if [[ "$target_dir" != /* ]]; then
    target_dir="$repository_root/$target_dir"
fi
bundle_dir="$target_dir/examples/GPUI Window Lifecycle.app"
mkdir -p "$bundle_dir/Contents/MacOS"
cp "$target_dir/debug/examples/window_lifecycle" "$bundle_dir/Contents/MacOS/window_lifecycle"
cat > "$bundle_dir/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
    <key>CFBundleIdentifier</key><string>dev.zed.gpui.window-lifecycle.prototype</string>
    <key>CFBundleName</key><string>GPUI Window Lifecycle</string>
    <key>CFBundleDisplayName</key><string>GPUI Window Lifecycle</string>
    <key>CFBundleExecutable</key><string>window_lifecycle</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>LSMinimumSystemVersion</key><string>15.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSUIElement</key><false/>
</dict></plist>
PLIST
codesign --force --sign - --timestamp=none "$bundle_dir"
printf 'Built ad hoc bundle: %s\n' "$bundle_dir"
