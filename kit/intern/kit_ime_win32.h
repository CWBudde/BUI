/* Input methods on Win32: a C port of Blender's GHOST_ImeWin32 (intern/ghost/intern/
 * GHOST_ImeWin32.cc, from Chromium's ImeInput): the IMM32 calls that enable an IME on a window,
 * place its candidate window, and read the composition and result strings.
 *
 * Message handling in the window procedure follows GHOST_SystemWin32.cc:
 *   WM_INPUTLANGCHANGE        bk_ime_win32_update_input_language + update_conversion_status
 *   WM_IME_NOTIFY             update_conversion_status (open status / conversion mode changes)
 *   WM_IME_SETCONTEXT         update_input_language, update_conversion_status, create_window,
 *                             cleanup_composition, check_first; then DefWindowProc
 *   WM_IME_STARTCOMPOSITION   create_window, reset_composition; handled (no DefWindowProc)
 *   WM_IME_COMPOSITION        update_window, update_info; handled
 *   WM_IME_ENDCOMPOSITION     reset_composition, destroy_window; handled
 * and the kit asks for begin/end (wm_window.cc WM_window_IME_begin / _end) through
 * BK_ime_state.
 *
 * SPDX-FileCopyrightText: 2010 The Chromium Authors. All rights reserved.
 * SPDX-License-Identifier: GPL-2.0-or-later */

#pragma once

#ifdef _WIN32

#  include <stdbool.h>
#  include <windows.h>

#  include "BUI_kit.h"

/* MSDN LOCALE_SISO639LANGNAME: at most 9 characters including the terminator. */
#  define BK_IME_ISO639_LEN 9

/* ImeComposition: the status of an ongoing composition, as UTF-16 straight from IMM32. */
typedef struct BKImeComposition {
  int cursor_position; /* the IME caret in the composition string */
  int target_start;    /* the clause being converted: start and end */
  int target_end;
  wchar_t *string; /* NUL terminated */
  int length, cap;
} BKImeComposition;

typedef struct BKImeWin32 {
  bool is_composing;
  char language[BK_IME_ISO639_LEN]; /* "en", "zh", "ja", "ko", ... */
  DWORD conversion_modes;
  DWORD sentence_mode;
  bool system_caret; /* a temporary system caret exists, for IMEs that read ::GetCaretPos */
  int caret_l, caret_t, caret_r, caret_b;
  bool is_first, is_enable;

  BKImeComposition result, comp;
  /* GHOST_TEventImeData: what the last composition event carried, as UTF-8 */
  char *result_utf8, *composite_utf8;
  int result_cap, composite_cap;
  int cursor_pos, target_start, target_end;
  DWORD *clauses; /* GCS_COMPCLAUSE scratch */
  int clauses_cap;
} BKImeWin32;

void bk_ime_win32_init(BKImeWin32 *ime);
void bk_ime_win32_free(BKImeWin32 *ime);

void bk_ime_win32_update_input_language(BKImeWin32 *ime);
void bk_ime_win32_update_conversion_status(BKImeWin32 *ime, HWND hwnd);
bool bk_ime_win32_is_english_mode(const BKImeWin32 *ime);
/* Whether an IME handles this character key itself (`ascii` as typed, `numpad_period` for the
 * numpad's decimal key), so the window must not also type it. */
bool bk_ime_win32_is_ime_key_event(const BKImeWin32 *ime, char ascii, bool numpad_period);

void bk_ime_win32_create_window(BKImeWin32 *ime, HWND hwnd);
void bk_ime_win32_destroy_window(BKImeWin32 *ime, HWND hwnd);
void bk_ime_win32_update_window(BKImeWin32 *ime, HWND hwnd);
void bk_ime_win32_cleanup_composition(BKImeWin32 *ime, HWND hwnd);
void bk_ime_win32_reset_composition(BKImeWin32 *ime);
void bk_ime_win32_check_first(BKImeWin32 *ime, HWND hwnd);

/* Enables the IME (the default input context) for the window and moves its candidate window to
 * the caret rectangle; `complete` also completes an ongoing composition first, as GHOST's
 * beginIME does when another field takes over. */
void bk_ime_win32_begin(BKImeWin32 *ime, HWND hwnd, int l, int t, int r, int b, bool complete);
/* Disables the IME for the window, cancelling an ongoing composition. */
void bk_ime_win32_end(BKImeWin32 *ime, HWND hwnd);

/* Reads the result and composition strings into `event` (UTF-8; valid until the next call). */
void bk_ime_win32_update_info(BKImeWin32 *ime, HWND hwnd, BKImeEvent *event);
/* The state of "no composition": empty strings, no caret, no clause. */
void bk_ime_win32_empty_event(BKImeWin32 *ime, BKImeEvent *event);

#endif /* _WIN32 */
