// SPDX-License-Identifier: GPL-2.0-or-later

//go:build bui_cgo

package bui

import (
	"github.com/ppmpreetham/BUI/bindings/go/internal/engine"
	"github.com/ppmpreetham/BUI/bindings/go/internal/engine/cgoengine"
)

var (
	kit  engine.Kit  = cgoengine.Kit{}
	host engine.Host = cgoengine.Host{}
)
