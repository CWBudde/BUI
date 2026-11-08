// SPDX-License-Identifier: GPL-2.0-or-later

//go:build !linux && !bui_cgo

package bui

import (
	"github.com/ppmpreetham/BUI/bindings/go/internal/engine"
	"github.com/ppmpreetham/BUI/bindings/go/internal/engine/wasmengine"
)

var host engine.Host = wasmengine.Host{}
