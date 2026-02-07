.PHONY: all build build-release test lint fmt fmt-fix clean build-ffi build-ffi-debug build-app help

# Default target
all: lint test build

# Build debug
build:
	cargo build --workspace

# Build release
build-release:
	cargo build --release --workspace

# Run all tests
test:
	cargo test --workspace

# Run clippy linter
lint:
	cargo clippy --workspace -- -D warnings

# Check formatting
fmt:
	cargo fmt --workspace -- --check

# Format code (fix)
fmt-fix:
	cargo fmt --workspace

# Build FFI bindings (requires macOS)
build-ffi:
	bash scripts/build-ffi.sh release

build-ffi-debug:
	bash scripts/build-ffi.sh debug

# Build Swift app (requires Xcode)
build-app: build-ffi
	xcodebuild -project app/MacAgentWatch/MacAgentWatch.xcodeproj -scheme MacAgentWatch -configuration Release build

# Clean all build artifacts
clean:
	cargo clean
	rm -rf app/MacAgentWatchCore/lib app/MacAgentWatchCore/include app/MacAgentWatchCore/generated
	rm -rf app/MacAgentWatch/MacAgentWatch/Generated

# Show help
help:
	@echo "MacAgentWatch Build Targets:"
	@echo "  make            - Run lint, test, build (default)"
	@echo "  make build      - Build debug"
	@echo "  make build-release - Build release"
	@echo "  make test       - Run all tests"
	@echo "  make lint       - Run clippy"
	@echo "  make fmt        - Check formatting"
	@echo "  make fmt-fix    - Fix formatting"
	@echo "  make build-ffi  - Build FFI bindings (release)"
	@echo "  make build-app  - Build Swift app"
	@echo "  make clean      - Clean all artifacts"
	@echo "  make help       - Show this help"
