# BUI

A modular UI toolkit extracted from the Blender project. Core in C, widgets in
C++, no Blender application logic — just the interface machinery.

See [ARCHITECTURE.md](ARCHITECTURE.md) for the module layout, dependency
rules, and the exact decoupling status.

## Quick start

```sh
cmake -S . -B build
cmake --build build
ctest --test-dir build        # core foundation tests pass
```

## Modules

- `core/`   – pure C11 foundation (containers, memory, math, strings)
- `types/`  – shared data-type shapes (DNA structs, verbatim)
- `gpu/`    – drawing abstraction + null backend (plug your renderer here)
- `compat/` – seam headers replacing Blender-app APIs (RNA/WM/ED/BLF/IMB/CTX)
- `ui/`     – the widget toolkit (blocks/buttons, layout, handlers, views)

## License

Derived from Blender sources: GPL-2.0-or-later (see upstream `COPYING` and the
SPDX headers preserved in every file).
