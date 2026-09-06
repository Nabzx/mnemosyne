# Mnemosyne task runner. `just` lists these; `just <name>` runs one.
# See CONTRIBUTING.md.

set shell := ["bash", "-uc"]

_default:
    @just --list

# Everything CI runs, in the same order. Run this before opening a pull request.
ci: fmt-check clippy test build deny py checks commits

# Format the whole workspace.
fmt:
    cargo fmt --all

# Check formatting without changing anything (CI does this).
fmt-check:
    cargo fmt --all --check

# Clippy with warnings as errors, every target and feature.
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# The Rust tests. `mnem-py` is excluded: its test binary needs libpython.
test:
    cargo test --workspace --exclude mnem-py

# Release build of the shipping crates.
build:
    cargo build --workspace --exclude mnem-py --release

# The MSRV check, at Rust 1.85.
msrv:
    RUSTUP_TOOLCHAIN=1.85.0 cargo check --all --all-features

# `mnem-core` must stay offline: no network, TLS or async-runtime crate.
deny:
    cargo deny check bans

# Build the extension, then lint and test the Python side.
py:
    maturin develop -m crates/mnem-py/Cargo.toml
    ruff check python scripts
    pytest python/tests scripts -q

# Build the release wheel and check it against the SDK tests.
wheel:
    rm -rf dist
    maturin build --release --out dist
    python -c "import zipfile,glob; z=zipfile.ZipFile(glob.glob('dist/*.whl')[0]); n=z.namelist(); assert 'mnem/_mnem.pyi' in n and 'mnem/py.typed' in n, n"

# Repo consistency: the ADR index, ADR references, and the changelog.
checks:
    python scripts/check_adr_index.py
    python scripts/check_adr_refs.py
    python scripts/check_changelog.py --range main..HEAD

# Check that this branch's commits follow the conventional-commit format.
commits:
    python scripts/conventional_commits.py --range main..HEAD

# Install the repo git hooks into .git/hooks.
install-hooks:
    git config core.hooksPath .githooks
    @echo "hooks installed: .githooks is now core.hooksPath"
