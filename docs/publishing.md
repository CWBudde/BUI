# Packaging and publishing (`blenderui` on PyPI)

| | |
|---|---|
| PyPI name | **`blenderui`** (`pip install blenderui`) |
| Import names | `blenderui` (recommended) and `bui` (the same modules), plus the extension `buipy` |
| Build backend | [scikit-build-core](https://scikit-build-core.readthedocs.io) driving the project's CMake (`pyproject.toml`) |
| Version | `bindings/python/bui/_version.py`, the single source (CMake reads it too) |
| Python | 3.11+ (CPython) |
| Console script | `blenderui-tk app.py`, which is the same as `python -m bui.tksyntax app.py` |

> The import name `bui` is kept for existing code. An unrelated project owns
> `bui` on PyPI, so published code should use `import blenderui`. The two
> can't both be installed under the same import name.

## What a wheel contains

```
site-packages/
  buipy.<abi>.pyd|.so     the C kit + raster engine (one extension module)
  bui/                     tksyntax/, qtsyntax.py, components/, host.py ...
  blenderui/               alias package: blenderui.X is bui.X
  blenderui-<v>.dist-info/
```

On Windows the extension is built with MinGW-w64 GCC (UCRT) and statically
linked against the GCC runtime. It depends only on system DLLs and
`python3xx.dll`. A wheel is about 0.4 MB compressed; the sdist is about
1.7 MB (it excludes `reference/` and build trees).

## Build locally

```sh
python -m pip install build
python -m build                    # sdist + wheel in dist/

# Windows with MSYS2 MinGW (the tested toolchain):
set CMAKE_GENERATOR=MinGW Makefiles
set CC=gcc
set CXX=g++
python -m build
```

Test the wheel in a clean environment:

```sh
python -m venv /tmp/bui-venv
/tmp/bui-venv/bin/pip install dist/blenderui-*.whl
/tmp/bui-venv/bin/python -c "import blenderui, blenderui.tksyntax, blenderui.components; print(blenderui.__version__)"
```

The test suites (`bindings/python/tests/`) pass against an installed wheel.
Remove the source tree from `sys.path` and keep `examples/` on it for
`test_components`.

## CI (`.github/workflows/wheels.yml`)

| Job | Does |
|---|---|
| `wheels` | `cibuildwheel` for Linux (manylinux x86_64), Windows (MinGW, x86_64) and macOS (arm64 + x86_64), CPython 3.11–3.14; each wheel is imported as a smoke test |
| `sdist` | `python -m build --sdist` |
| `tests` | CMake build + `ctest` (all C and Python suites) on Windows |
| `publish` | On a `v*` tag: upload everything to PyPI with trusted publishing. Manual run with `testpypi`: upload to TestPyPI. |

## One-time PyPI setup

1. Create the `blenderui` project on PyPI. Register it as a
   [pending trusted publisher](https://docs.pypi.org/trusted-publishers/)
   with this repository, workflow `wheels.yml`, environment `pypi`.
2. Do the same on TestPyPI with environment `testpypi`.
3. In the GitHub repository settings, create the environments `pypi` and
   `testpypi` (optionally with required reviewers).
4. Replace `OWNER` in `pyproject.toml` `[project.urls]` with the GitHub
   owner.

No API tokens are stored anywhere.

## Release checklist

1. Bump `bindings/python/bui/_version.py` (semantic versioning).
2. Add the release to `CHANGELOG.md`.
3. `ctest --test-dir build` passes locally. Optionally run the workflow by
   hand with `testpypi` and `pip install -i https://test.pypi.org/simple blenderui`.
4. `git tag v0.1.0 && git push --tags`. CI builds, tests and publishes.

## Known gaps before 1.0

- Linux and macOS wheels build in CI but haven't been exercised
  interactively. Their window host is the stdlib Tk window, so the system
  Python needs `tkinter` (for example `python3-tk`).
- MSVC builds: the forced-include shim has an MSVC (`/FI`) path, but MSVC
  isn't tested, which is why CI uses MinGW on Windows.
- No `musllinux` or 32-bit wheels yet (skipped in `[tool.cibuildwheel]`).
- The README's images use relative paths. PyPI shows them once
  `[project.urls]` points to the public repository and the paths are made
  absolute (or the images are hosted).

# Publishing the Rust crates (crates.io)

The four crates are published in dependency order:

| Crate | What it is | Depends on |
|---|---|---|
| `blenderui-tailwind` | Tailwind v4 class compiler (std only) | none |
| `blenderui-macros` | `rsx!` / `view!`, `#[component]`, `css!`, `tw!` | `blenderui-tailwind` |
| `blenderui-sys` | FFI over the C kit; builds it with CMake | none (`cmake` at build time) |
| `blenderui` | the framework | all three |

`blenderui-sys` must carry the C sources, because a published crate can't
reach `../../..`. Vendor them first. They're git-ignored and listed in the
crate's `include`:

```sh
python bindings/rust/scripts/vendor_c.py          # copies ~7 MB into blenderui-sys/csrc/
cd bindings/rust
cargo package --workspace --allow-dirty           # builds each packaged crate in isolation
```

`build.rs` looks for the C sources in three places, in order:
1. `BUI_SOURCE_DIR`, if set;
2. the surrounding repository;
3. `csrc/`, the vendored copy (what a downloaded crate has).

## Release

1. Bump `version` in `bindings/rust/Cargo.toml` (`[workspace.package]`). The
   crates share one version. Then update the `version = "..."` of the path
   dependencies in `blenderui/Cargo.toml` and `blenderui-macros/Cargo.toml`.
2. `cargo login` once with a crates.io API token.
3. Vendor, then publish in order. `cargo publish --workspace` does the
   ordering itself on Cargo 1.90+:

```sh
python bindings/rust/scripts/vendor_c.py
cd bindings/rust
cargo publish --workspace --allow-dirty   # or, one by one:
# cargo publish -p blenderui-tailwind
# cargo publish -p blenderui-macros
# cargo publish -p blenderui-sys --allow-dirty
# cargo publish -p blenderui
```

`--allow-dirty` is needed because `csrc/` is untracked. Users need CMake and
a C compiler (MSVC, MinGW, gcc or clang) to build `blenderui-sys`.
`bench/` is excluded from the workspace and is never published.
