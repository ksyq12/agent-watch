#!/bin/bash
set -euo pipefail

# Build FFI artifacts for MacAgentWatch
# Usage: bash scripts/build-ffi.sh [debug|release]

# Ensure Rust toolchain is in PATH (Xcode strips user PATH)
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
elif [ -d "$HOME/.cargo/bin" ]; then
    export PATH="$HOME/.cargo/bin:$PATH"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BUILD_MODE="${1:-release}"

echo "=== MacAgentWatch FFI Build ==="
echo "Project root: $PROJECT_ROOT"
echo "Build mode: $BUILD_MODE"

# Prerequisite checks
echo ""
echo "--- Checking prerequisites ---"
MISSING=""
for cmd in cargo rustc; do
    if ! command -v "$cmd" &>/dev/null; then
        MISSING="$MISSING $cmd"
    fi
done
if [ -n "$MISSING" ]; then
    echo "ERROR: Missing required tools:$MISSING"
    echo "Please install Rust toolchain: https://rustup.rs"
    exit 1
fi
echo "All prerequisites found."

# Output directories
CORE_DIR="$PROJECT_ROOT/app/MacAgentWatchCore"
LIB_DIR="$CORE_DIR/lib"
INCLUDE_DIR="$CORE_DIR/include"
GENERATED_DIR="$CORE_DIR/generated"

mkdir -p "$LIB_DIR" "$INCLUDE_DIR" "$GENERATED_DIR"

# Step 1: Build Rust static library
echo ""
echo "--- Step 1: Building Rust static library ($BUILD_MODE) ---"
if [ "$BUILD_MODE" = "release" ]; then
    cargo build --release -p macagentwatch-core
    TARGET_DIR="$PROJECT_ROOT/target/release"
else
    cargo build -p macagentwatch-core
    TARGET_DIR="$PROJECT_ROOT/target/debug"
fi

STATIC_LIB="$TARGET_DIR/libmacagentwatch_core.a"
if [ ! -f "$STATIC_LIB" ]; then
    echo "ERROR: Static library not found at $STATIC_LIB"
    exit 1
fi
echo "Static library: $STATIC_LIB"

# Step 2: Generate Swift bindings via uniffi-bindgen
echo ""
echo "--- Step 2: Generating Swift bindings ---"
# uniffi-bindgen needs the cdylib to extract metadata
DYLIB="$TARGET_DIR/libmacagentwatch_core.dylib"
if [ ! -f "$DYLIB" ]; then
    echo "ERROR: Dynamic library not found at $DYLIB"
    exit 1
fi
cargo run -p macagentwatch-core --bin uniffi-bindgen generate \
    --library "$DYLIB" \
    --language swift \
    --out-dir "$GENERATED_DIR"

echo "Generated files:"
ls -la "$GENERATED_DIR/"

# Step 3: Copy artifacts
echo ""
echo "--- Step 3: Copying artifacts ---"

# Copy static library
cp "$STATIC_LIB" "$LIB_DIR/"
echo "Copied: $LIB_DIR/libmacagentwatch_core.a"

# Copy header file
if [ -f "$GENERATED_DIR/macagentwatch_coreFFI.h" ]; then
    cp "$GENERATED_DIR/macagentwatch_coreFFI.h" "$INCLUDE_DIR/"
    echo "Copied: $INCLUDE_DIR/macagentwatch_coreFFI.h"
fi

# Create/update module map
cat > "$INCLUDE_DIR/module.modulemap" << 'MODULEMAP'
module macagentwatch_coreFFI {
    header "macagentwatch_coreFFI.h"
    export *
}
MODULEMAP
echo "Created: $INCLUDE_DIR/module.modulemap"

# Copy generated Swift bindings into Xcode source tree (auto-detected by file system sync)
SWIFT_GENERATED_DIR="$PROJECT_ROOT/app/MacAgentWatch/MacAgentWatch/Generated"
mkdir -p "$SWIFT_GENERATED_DIR"
cp "$GENERATED_DIR/macagentwatch_core.swift" "$SWIFT_GENERATED_DIR/"
echo "Copied: $SWIFT_GENERATED_DIR/macagentwatch_core.swift"

echo ""
echo "=== Build complete ==="
echo "Static library: $LIB_DIR/libmacagentwatch_core.a"
echo "Header: $INCLUDE_DIR/macagentwatch_coreFFI.h"
echo "Module map: $INCLUDE_DIR/module.modulemap"
echo "Swift bindings: $GENERATED_DIR/"
ls "$GENERATED_DIR/"*.swift 2>/dev/null || echo "(no .swift files yet)"
