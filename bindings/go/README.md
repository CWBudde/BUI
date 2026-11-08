# BUI for Go

Experimental Go bindings for BUI, which provides Blender's UI widgets as a C library. The module is
`github.com/ppmpreetham/BUI/bindings/go`, and package `bui` drives BUI's C kit: nodes, properties, geometry, rendering and
input. It was developed as [bui-go](https://github.com/CWBudde/bui-go) and moved here.

**Status: experimental.** The API is small and will change. This first part has the wasm engine, the public API and headless
rendering. Its window hosts follow, so by default it does not open windows yet: see [Limits](#limits).

## Try it

You need Go 1.26 or newer, and no C toolchain. `Headless` renders a node tree into memory and takes synthetic input, on any
platform and with no display. In a module that requires `github.com/ppmpreetham/BUI/bindings/go`, this prints `clicked`:

```go
package main

import (
	"fmt"

	bui "github.com/ppmpreetham/BUI/bindings/go"
)

func main() {
	root := bui.NewNode("frame")
	defer root.Close()

	button := bui.NewNode("button")
	button.SetStr("text", "Add step")
	root.Add(button)
	button.Pack(bui.PackOptions{Fill: bui.FillX})
	button.OnClick(func() { fmt.Println("clicked") })

	view := bui.NewHeadless(root, 320, 120)
	defer view.Close()

	view.Render() // lays out the tree, so Rect is valid from here on
	x, y, w, h := button.Rect()
	view.Press(x+w/2, y+h/2)
	view.Release(x+w/2, y+h/2)
}
```

The module has no tagged release yet, so take it at a commit, for example `go get github.com/ppmpreetham/BUI/bindings/go@main`.

In a window program the same calls look like this:

```go
button := bui.NewNode("button")
button.SetStr("text", "Add step")
root.Add(button)
button.Pack(bui.PackOptions{Fill: bui.FillX})
button.OnClick(func() { /* runs on the UI thread */ })

bui.Do(func() { clock.SetStr("text", "updated") }) // safe from any goroutine
```

`examples/counter` is such a program. With the default engine it exits with `ErrNoDisplay`. With the cgo engine on Linux, it
opens a real window:

```sh
go run -tags bui_cgo ./examples/counter   # needs BUI's libraries, see "Engines"
```

## Engines

The same API runs on two engines, chosen when you build:

| Build                    | Engine                                                                                  | Windows                                               | Needs                                      |
| ------------------------ | --------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------ |
| Default, on any platform | WebAssembly: an embedded module, in [wazero](https://github.com/tetratelabs/wazero)     | None yet: `NewWindow` and `Run` report `ErrNoDisplay` | Nothing but Go                             |
| `-tags bui_cgo`, Linux   | Native: the static libraries of BUI's CMake build, in `build/` at the repository's root | X11, and Wayland on request, through BUI's own hosts  | cgo, a C and C++ toolchain, and that build |

Both engines draw the same pixels: the tests compare them exactly.

**Building the cgo engine.** It links `libbui_kit.a`, `libbui_gpu.a` and `libbui_core.a` from `build/kit`, `build/gpu` and
`build/core` at the root of this repository, with the headers in `kit/include` and `gpu/include`. Build them at the root, then
use the tag:

```sh
cmake -S . -B build -DBUI_BUILD_TESTS=OFF -DBUI_BUILD_EXAMPLES=OFF
cmake --build build --target bui_kit
cd bindings/go
go test -tags bui_cgo ./...
```

Without the X11 headers (`libx11-dev`) CMake builds the kit with no window host, and `Supported` then stays false. The Wayland
host also needs `libwayland-dev`, `libxkbcommon-dev` and `wayland-protocols`. The tag fails to build without cgo or off Linux,
rather than falling back to the wasm engine.

**Wayland.** BUI's libraries can open native Wayland windows, and the kit picks them whenever `WAYLAND_DISPLAY` is set. The cgo
engine sets `BUI_WINDOW_BACKEND=x11` for them when the variable is unset, so its windows are X11 ones, which a Wayland desktop
provides through XWayland, as the Go window hosts that follow will need. Set `BUI_WINDOW_BACKEND=wayland` to get native Wayland
windows from the cgo engine. The wasm engine ignores the variable.

**Speed.** On amd64 and arm64 wazero compiles the module, which renders at about 2-4x native time and handles a hover at
about 1.2-3x. A dense 1920x1080 frame rendered in about 8-10 ms on the amd64 laptop of those measurements, when it was not loaded. Other architectures use wazero's
interpreter, at 100-1000x native: a small tree renders in about 36 ms and a dense full-HD frame in over a second, so they
are for correctness rather than interaction. [`docs/design.md`](docs/design.md) has the measurements, and `scripts/bench.sh`
records new ones.

**Compilation cache.** The first start compiles the module, which takes about 0.2-0.3 s. The result is cached in
`os.UserCacheDir()/bui-go/wazero`, which brings later starts to about 25 ms. A cache that cannot be read or written costs only
that time. The module loads at the first kit call, so importing the package costs nothing.

## Limits

- **No windows yet with the default engine.** The wasm engine has no window host in this tree: `Supported` is false, and
  `NewWindow` and `Run` report `ErrNoDisplay` on every platform. Only the cgo engine opens windows, through BUI's own hosts.
- **Software rendering only.** `Headless` draws into a CPU raster on both engines, and the wasm engine has no GPU renderer.
- **No GPU, video, file browser or IME.** The module is built without them, and the binding exposes none of them.
- **A wasm trap ends the process.** After a fault in the module its state is unknown, so any trap, an out-of-memory included,
  ends the process with a message rather than returning a Go error.

### Limits of the wasm engine

- No worker threads. Image sources would stay pending forever, though the API sets none today.
- Only the embedded Inter font works by default.
- `EvalNumber` accepts an expression whose intermediate inf or nan turns finite again, such as `1/(1/0)`, as wasm has no
  floating-point exception flags.
- No guard pages: inside the module a NULL read or a bad offset returns what memory holds, where native code would crash.
- Closing a context after an edit loses the edited field's old text, in the kit's own code, on either engine.
- Linear memory never shrinks, so the largest raster a process had stays in its footprint. Past 1 GiB the process ends with
  a message, as it does on any fault in the module.
- It depends on an experimental wazero feature, so wazero is pinned, and upgrades are deliberate changes.

## Threading

Call `bui.Run` from the main goroutine. Kit calls are valid only on the UI thread: the main goroutine before and after
`Run`, and handlers and `Do` functions during it. Other goroutines reach the UI thread through `bui.Do`.

## Known limits of the API

- The binding exposes generic properties (`SetNum`, `SetStr`), not typed widgets.
- `Run` on the main goroutine is documented, not enforced.
- `Do` calls made after the last `Run` stay queued, and a leftover `Do(bui.Quit)` ends the next `Run` immediately.
- Continuous rendering could delay `Do` on the native engine.

## Tests

```sh
CGO_ENABLED=0 xvfb-run -a go test ./...                # the wasm engine, with no cgo at all
xvfb-run -a go test -race ./...                        # the wasm engine, with the race detector
xvfb-run -a go test -race -tags bui_cgo ./...          # the cgo engine, compared with the wasm one
BUI_TEST_XDOTOOL=1 xvfb-run -a go test -race -tags bui_cgo -run '^TestX' .
./scripts/check-wasm.sh                                # rebuild bui_kit.wasm and require the committed bytes
```

Tests that need a window are skipped when there is no display, and on the wasm engine, which has none. `BUI_TEST_XDOTOOL=1`
sends real clicks and keys and changes the keyboard layout, so use a display of its own. It needs the cgo engine for now: on the
wasm engine those tests fail, as they find no display. `TestXTypingOnAGermanLayout` is for the Go X11 host, which is not here yet.

`TestSjljFreeTypeOverflow` builds its probe with `scripts/build-wasm.sh probe`, which needs the pinned wasi-sdk and this
repository's history. It skips without the SDK, which `scripts/build-wasm.sh sdk` fetches on Linux x86_64. The image tests read
`tests/images` from the repository's root and skip without it.

## Development

```sh
just setup-deps   # treefmt, prettier, gofumpt, gci, shfmt
just fmt          # format with treefmt
just lint         # golangci-lint, at the version pinned in the justfile, also with -tags bui_cgo
just check        # formatting, go.mod tidiness, lint and tests
```

CI runs the formatting, the linter, the tests without cgo on Linux and macOS, and `check-wasm`, in
[`.github/workflows/go.yml`](../../.github/workflows/go.yml).

## Licence and source of the binary

The licence statements in this section are the project's own reading of the licences involved. They have had no legal review
and are not legal advice, so check them for your own case before you distribute a program built on this binding.

The binding follows BUI's licensing. As a whole it is GPL-3.0-or-later, and BUI's [`LICENSE`](../../LICENSE) holds the GPLv3
text. Its files carry the SPDX header `GPL-2.0-or-later`, as BUI's files do. We read the GPL as applying to a program that links
this package.

**We read a binary with the wasm engine as distributable under GPLv3,** as the whole is: it includes wazero, which is Apache-2.0,
and as we understand it that licence is compatible with GPLv3 but not with GPLv2.
[`THIRD_PARTY_NOTICES.md`](../../THIRD_PARTY_NOTICES.md) lists the third-party parts and their licences, under "The Go binding",
and [`licenses/`](licenses) holds the texts that only the binding brings.

The only committed binary is `internal/engine/wasmengine/bui_kit.wasm`. Its corresponding source is BUI at the commit in
[`BUI_COMMIT`](BUI_COMMIT), which is in this repository's history, `wasm/` here, and wasi-sdk 34.0, whose version and checksum
are in `scripts/build-wasm.sh`, which builds it:

```sh
scripts/build-wasm.sh   # takes BUI_COMMIT's sources with git archive, downloads the SDK into the gitignored third_party/
```

Build paths are stripped from the output, and the module rebuilds byte-identically from another directory. CI rebuilds it and
requires identical bytes. Moving the binding to a newer BUI means changing `BUI_COMMIT`, rebuilding, and checking the pinned
pixel hashes in `internal/enginetest`. The cgo engine's libraries are not committed: they come from this tree's CMake build.
