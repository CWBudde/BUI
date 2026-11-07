/* Input methods on Win32: a C port of Blender's GHOST_ImeWin32 (intern/ghost/intern/
 * GHOST_ImeWin32.cc, which is Chromium's ImeInput). The function names follow GHOST's methods;
 * see kit_ime_win32.h for how the window procedure uses them.
 *
 * SPDX-FileCopyrightText: 2010 The Chromium Authors. All rights reserved.
 * SPDX-License-Identifier: GPL-2.0-or-later */

#ifdef _WIN32

#  include <stdlib.h>
#  include <string.h>

#  include <windows.h>

#  include <imm.h>

#  include "kit_ime_win32.h"

#  ifdef _MSC_VER
#    pragma comment(lib, "imm32")
#  endif

/* ISO_639-1 2-Letter Abbreviations. */
#  define IMELANG_ENGLISH "en"
#  define IMELANG_CHINESE "zh"
#  define IMELANG_JAPANESE "ja"
#  define IMELANG_KOREAN "ko"

void bk_ime_win32_init(BKImeWin32 *ime)
{
  memset(ime, 0, sizeof(*ime));
  strcpy(ime->language, IMELANG_ENGLISH);
  ime->conversion_modes = IME_CMODE_ALPHANUMERIC;
  ime->sentence_mode = IME_SMODE_NONE;
  ime->caret_l = ime->caret_t = -1;
  ime->is_first = true;
  ime->is_enable = true;
  ime->cursor_pos = ime->target_start = ime->target_end = -1;
}

void bk_ime_win32_free(BKImeWin32 *ime)
{
  free(ime->result.string);
  free(ime->comp.string);
  free(ime->result_utf8);
  free(ime->composite_utf8);
  free(ime->clauses);
  memset(ime, 0, sizeof(*ime));
}

static bool is_language(const BKImeWin32 *ime, const char *name)
{
  return strcmp(name, ime->language) == 0;
}

void bk_ime_win32_update_input_language(BKImeWin32 *ime)
{
  /* Get the current input locale full name. */
  WCHAR locale[LOCALE_NAME_MAX_LENGTH];
  LCIDToLocaleName(MAKELCID(LOWORD(GetKeyboardLayout(0)), SORT_DEFAULT), locale,
                   LOCALE_NAME_MAX_LENGTH, 0);
  /* Get the 2-letter ISO-63901 abbreviation of the input locale name. */
  WCHAR language_u16[BK_IME_ISO639_LEN];
  GetLocaleInfoEx(locale, LOCALE_SISO639LANGNAME, language_u16, BK_IME_ISO639_LEN);
  /* Store this as a UTF8 string. */
  WideCharToMultiByte(CP_UTF8, 0, language_u16, BK_IME_ISO639_LEN, ime->language,
                      BK_IME_ISO639_LEN, NULL, NULL);
  ime->language[BK_IME_ISO639_LEN - 1] = '\0';
}

void bk_ime_win32_update_conversion_status(BKImeWin32 *ime, HWND hwnd)
{
  HIMC imm_context = ImmGetContext(hwnd);
  if (imm_context) {
    if (ImmGetOpenStatus(imm_context)) {
      ImmGetConversionStatus(imm_context, &ime->conversion_modes, &ime->sentence_mode);
    }
    else {
      ime->conversion_modes = IME_CMODE_ALPHANUMERIC;
      ime->sentence_mode = IME_SMODE_NONE;
    }
    ImmReleaseContext(hwnd, imm_context);
  }
  else {
    ime->conversion_modes = IME_CMODE_ALPHANUMERIC;
    ime->sentence_mode = IME_SMODE_NONE;
  }
}

bool bk_ime_win32_is_english_mode(const BKImeWin32 *ime)
{
  return (ime->conversion_modes & IME_CMODE_NOCONVERSION) ||
         !(ime->conversion_modes & (IME_CMODE_NATIVE | IME_CMODE_FULLSHAPE));
}

bool bk_ime_win32_is_ime_key_event(const BKImeWin32 *ime, char ascii, bool numpad_period)
{
  if (!bk_ime_win32_is_english_mode(ime)) {
    /* In Chinese, Japanese, Korean, all alpha keys are processed by IME. */
    if ((ascii >= 'A' && ascii <= 'Z') || (ascii >= 'a' && ascii <= 'z')) {
      return true;
    }
    if (is_language(ime, IMELANG_JAPANESE) && (ascii >= ' ' && ascii <= '~')) {
      return true;
    }
    if (is_language(ime, IMELANG_CHINESE)) {
      if (ascii && strchr("!\"$'(),.:;<>?[\\]^_`/", ascii) && !numpad_period) {
        return true;
      }
      if ((ime->conversion_modes & IME_CMODE_FULLSHAPE) && (ascii >= '0' && ascii <= '9')) {
        /* When in Full Width mode the number keys are also converted. */
        return true;
      }
    }
  }
  return false;
}

static void move_window(BKImeWin32 *ime, HIMC imm_context)
{
  int x = ime->caret_l;
  int y = ime->caret_t;
  const int caret_margin = 1;
  /* Chinese IMEs ignore ImmSetCandidateWindow when TSF and CUAS are disabled and read the
   * system caret instead; with them enabled they ignore the caret and use CFS_CANDIDATEPOS.
   * So set both. */
  CANDIDATEFORM candidate_position = {0, CFS_CANDIDATEPOS, {x, y}, {0, 0, 0, 0}};
  ImmSetCandidateWindow(imm_context, &candidate_position);
  if (ime->system_caret) {
    SetCaretPos(x, y);
  }
  if (is_language(ime, IMELANG_KOREAN)) {
    /* Chinese and Japanese IMEs want the upper-left corner of the caret to move their candidate
     * windows, Korean IMEs the lower-left one. */
    y += caret_margin;
  }
  /* Japanese and Korean IMEs also use CFS_EXCLUDE to place their candidate windows when TSF and
   * CUAS are disabled. */
  CANDIDATEFORM exclude_rectangle = {
      0, CFS_EXCLUDE, {x, y},
      {x, y, x + (ime->caret_r - ime->caret_l), y + (ime->caret_b - ime->caret_t)}};
  ImmSetCandidateWindow(imm_context, &exclude_rectangle);
}

void bk_ime_win32_update_window(BKImeWin32 *ime, HWND hwnd)
{
  /* Just move the IME window attached to the given window. */
  if (ime->caret_l >= 0 && ime->caret_t >= 0) {
    HIMC imm_context = ImmGetContext(hwnd);
    if (imm_context) {
      move_window(ime, imm_context);
      ImmReleaseContext(hwnd, imm_context);
    }
  }
}

void bk_ime_win32_create_window(BKImeWin32 *ime, HWND hwnd)
{
  /* With TSF and CUAS disabled, Chinese IMEs ignore ImmSetCandidateWindow and use ::GetCaretPos,
   * and some Japanese IMEs do too: create a temporary system caret for this input context. */
  if (!ime->system_caret && (is_language(ime, IMELANG_CHINESE) || is_language(ime, IMELANG_JAPANESE))) {
    ime->system_caret = CreateCaret(hwnd, NULL, 1, 1) != 0;
  }
  /* Restore the positions of the IME windows. */
  bk_ime_win32_update_window(ime, hwnd);
}

void bk_ime_win32_destroy_window(BKImeWin32 *ime, HWND hwnd)
{
  (void)hwnd;
  /* Destroy the system caret if we have created for this IME input context. */
  if (ime->system_caret) {
    DestroyCaret();
    ime->system_caret = false;
  }
}

void bk_ime_win32_reset_composition(BKImeWin32 *ime)
{
  /* Currently, just reset the composition status. */
  ime->is_composing = false;
}

void bk_ime_win32_cleanup_composition(BKImeWin32 *ime, HWND hwnd)
{
  /* Complete the ongoing composition (the window was de-activated while composing and is
   * re-activated), and reset the composition status. */
  if (ime->is_composing) {
    HIMC imm_context = ImmGetContext(hwnd);
    if (imm_context) {
      ImmNotifyIME(imm_context, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);
      ImmReleaseContext(hwnd, imm_context);
    }
    bk_ime_win32_reset_composition(ime);
  }
}

static void complete_composition(BKImeWin32 *ime, HIMC imm_context)
{
  /* Only complete a composition that exists: some IMEs get confused otherwise. */
  if (ime->is_composing) {
    ImmNotifyIME(imm_context, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);
    bk_ime_win32_reset_composition(ime);
  }
}

void bk_ime_win32_end(BKImeWin32 *ime, HWND hwnd)
{
  /* The input focus moved to something that takes no IME (or another field) while composing:
   * cancel the composition and clean up BEFORE disabling the IME. */
  if (!ime->is_enable) {
    return;
  }
  ime->is_enable = false;
  if (ime->is_composing) {
    HIMC imm_context = ImmGetContext(hwnd);
    if (imm_context) {
      /* Cancel composition */
      ImmNotifyIME(imm_context, NI_COMPOSITIONSTR, CPS_CANCEL, 0);
      ImmNotifyIME(imm_context, NI_CLOSECANDIDATE, 0, 0);
      ImmReleaseContext(hwnd, imm_context);
    }
    bk_ime_win32_reset_composition(ime);
  }
  ImmAssociateContextEx(hwnd, NULL, 0);
  if (ime->composite_utf8) {
    ime->composite_utf8[0] = '\0';
  }
}

void bk_ime_win32_check_first(BKImeWin32 *ime, HWND hwnd)
{
  /* The IME starts disabled: it is on only while a field takes text. */
  if (ime->is_first) {
    bk_ime_win32_end(ime, hwnd);
    ime->is_first = false;
  }
}

void bk_ime_win32_begin(BKImeWin32 *ime, HWND hwnd, int l, int t, int r, int b, bool complete)
{
  if (ime->is_enable && complete) {
    return;
  }
  ime->is_enable = true;
  /* Load the default IME context. IMM ignores this call if the context is loaded. */
  ImmAssociateContextEx(hwnd, NULL, IACE_DEFAULT);
  /* Complete the ongoing composition and move the IME windows. */
  HIMC imm_context = ImmGetContext(hwnd);
  if (imm_context) {
    if (complete) {
      /* The focus moved to another edit control while composing: complete the composition and
       * hide the IME windows BEFORE MOVING THEM. */
      complete_composition(ime, imm_context);
    }
    /* Save the caret position, and update the position of the IME window. */
    if (l >= 0 && t >= 0) {
      ime->caret_l = l;
      ime->caret_t = t;
      ime->caret_r = r;
      ime->caret_b = b;
      move_window(ime, imm_context);
    }
    ImmReleaseContext(hwnd, imm_context);
  }
}

/* -------------------------------------------------------------------- */
/* Reading the strings */

static bool is_target_attribute(char attribute)
{
  return attribute == ATTR_TARGET_CONVERTED || attribute == ATTR_TARGET_NOTCONVERTED;
}

static void *grow(void *p, int *cap, int need, size_t elem)
{
  if (need > *cap) {
    *cap = need + 64;
    p = realloc(p, (size_t)*cap * elem);
  }
  return p;
}

/* Retrieve a string from the IMM. */
static bool get_string(HIMC imm_context, LPARAM lparam, int type, BKImeComposition *c)
{
  c->length = 0;
  if (lparam & type) {
    const int string_size = ImmGetCompositionStringW(imm_context, type, NULL, 0);
    if (string_size > 0) {
      const int string_length = string_size / (int)sizeof(wchar_t);
      c->string = (wchar_t *)grow(c->string, &c->cap, string_length + 1, sizeof(wchar_t));
      ImmGetCompositionStringW(imm_context, type, c->string, (DWORD)string_size);
      c->string[string_length] = L'\0';
      c->length = string_length;
      return true;
    }
  }
  return false;
}

/* The target area: language dependent, as the caret's style is. */
static void get_caret(BKImeWin32 *ime, HIMC imm_context, LPARAM lparam, BKImeComposition *c)
{
  int target_start = -1;
  int target_end = -1;
  if (is_language(ime, IMELANG_KOREAN)) {
    /* Korean IMEs: the caret is a blinking block (one hangul character). */
    if (lparam & CS_NOMOVECARET) {
      target_start = 0;
      target_end = 1;
    }
  }
  else if (is_language(ime, IMELANG_CHINESE)) {
    /* Chinese IMEs: the caret is a blinking line, no target selection needed... */
    int clause_size = ImmGetCompositionStringW(imm_context, GCS_COMPCLAUSE, NULL, 0);
    if (clause_size) {
      clause_size = clause_size / (int)sizeof(DWORD);
      ime->clauses = (DWORD *)grow(ime->clauses, &ime->clauses_cap, clause_size, sizeof(DWORD));
      ImmGetCompositionStringW(imm_context, GCS_COMPCLAUSE, ime->clauses,
                               (DWORD)(sizeof(DWORD) * (size_t)clause_size));
      if (clause_size >= 2 && c->cursor_position == c->length) {
        target_start = (int)ime->clauses[clause_size - 2];
        target_end = (int)ime->clauses[clause_size - 1];
      }
      else {
        for (int i = 0; i < clause_size - 1; i++) {
          if ((int)ime->clauses[i] == c->cursor_position) {
            target_start = (int)ime->clauses[i];
            target_end = (int)ime->clauses[i + 1];
            break;
          }
        }
      }
    }
    else if (c->cursor_position != -1) {
      target_start = c->cursor_position;
      target_end = c->length;
    }
  }
  else if (is_language(ime, IMELANG_JAPANESE)) {
    /* Japanese IMEs: the caret is a selection (or underlined) block. The robustest way to find
     * it is scanning the attribute of the latest composition string for the target clause, the
     * one being converted. */
    if (lparam & GCS_COMPATTR) {
      const int attribute_size = ImmGetCompositionStringW(imm_context, GCS_COMPATTR, NULL, 0);
      if (attribute_size > 0) {
        char *attribute_data = (char *)malloc((size_t)attribute_size);
        if (attribute_data) {
          ImmGetCompositionStringW(imm_context, GCS_COMPATTR, attribute_data,
                                   (DWORD)attribute_size);
          for (target_start = 0; target_start < attribute_size; ++target_start) {
            if (is_target_attribute(attribute_data[target_start])) {
              break;
            }
          }
          for (target_end = target_start; target_end < attribute_size; ++target_end) {
            if (!is_target_attribute(attribute_data[target_end])) {
              break;
            }
          }
          if (target_start == attribute_size) {
            /* No target clause: this is an input clause, treat the whole of it as the target. */
            target_end = target_start;
            target_start = 0;
          }
          if (target_start != -1 && target_start < attribute_size &&
              attribute_data[target_start] == ATTR_TARGET_NOTCONVERTED)
          {
            c->cursor_position = target_start;
          }
          free(attribute_data);
        }
      }
    }
  }
  c->target_start = target_start;
  c->target_end = target_end;
}

static bool get_result(BKImeWin32 *ime, HWND hwnd, LPARAM lparam, BKImeComposition *c)
{
  bool result = false;
  HIMC imm_context = ImmGetContext(hwnd);
  if (imm_context) {
    /* Copy the result string; it has no composition attributes, so reset the others. */
    result = get_string(imm_context, lparam, GCS_RESULTSTR, c);
    c->cursor_position = -1;
    c->target_start = -1;
    c->target_end = -1;
    ImmReleaseContext(hwnd, imm_context);
  }
  (void)ime;
  return result;
}

static bool get_composition(BKImeWin32 *ime, HWND hwnd, LPARAM lparam, BKImeComposition *c)
{
  bool result = false;
  HIMC imm_context = ImmGetContext(hwnd);
  if (imm_context) {
    /* Copy the composition string. */
    result = get_string(imm_context, lparam, GCS_COMPSTR, c);
    /* Retrieve the cursor position in the IME composition. */
    c->cursor_position = ImmGetCompositionStringW(imm_context, GCS_CURSORPOS, NULL, 0);
    c->target_start = -1;
    c->target_end = -1;
    /* Retrieve the target selection. */
    get_caret(ime, imm_context, lparam, c);
    /* Mark that there is an ongoing composition. */
    ime->is_composing = true;
    ImmReleaseContext(hwnd, imm_context);
  }
  return result;
}

/* UTF-16 index -> UTF-8 byte offset (convert_utf16_to_utf8_len), -1 when out of range. */
static int utf8_offset(const BKImeComposition *c, int len)
{
  if (len >= 0 && len <= c->length) {
    return len ? WideCharToMultiByte(CP_UTF8, 0, c->string, len, NULL, 0, NULL, NULL) : 0;
  }
  return -1;
}

static void to_utf8(const BKImeComposition *c, char **buf, int *cap)
{
  const int n = c->length ? WideCharToMultiByte(CP_UTF8, 0, c->string, c->length, NULL, 0, NULL,
                                                 NULL) :
                            0;
  *buf = (char *)grow(*buf, cap, n + 1, 1);
  if (n) {
    WideCharToMultiByte(CP_UTF8, 0, c->string, c->length, *buf, n, NULL, NULL);
  }
  (*buf)[n] = '\0';
}

void bk_ime_win32_empty_event(BKImeWin32 *ime, BKImeEvent *event)
{
  ime->result_utf8 = (char *)grow(ime->result_utf8, &ime->result_cap, 1, 1);
  ime->composite_utf8 = (char *)grow(ime->composite_utf8, &ime->composite_cap, 1, 1);
  ime->result_utf8[0] = ime->composite_utf8[0] = '\0';
  ime->cursor_pos = ime->target_start = ime->target_end = -1;
  event->result = ime->result_utf8;
  event->composite = ime->composite_utf8;
  event->cursor_pos = event->sel_start = event->sel_end = -1;
}

void bk_ime_win32_update_info(BKImeWin32 *ime, HWND hwnd, BKImeEvent *event)
{
  const bool res = get_result(ime, hwnd, GCS_RESULTSTR, &ime->result);
  const bool comp = get_composition(ime, hwnd, GCS_COMPSTR | GCS_COMPATTR, &ime->comp);
  bk_ime_win32_empty_event(ime, event);
  /* Convert wchar to UTF8. */
  if (res) {
    to_utf8(&ime->result, &ime->result_utf8, &ime->result_cap);
  }
  if (comp) {
    to_utf8(&ime->comp, &ime->composite_utf8, &ime->composite_cap);
    ime->cursor_pos = utf8_offset(&ime->comp, ime->comp.cursor_position);
    ime->target_start = utf8_offset(&ime->comp, ime->comp.target_start);
    ime->target_end = utf8_offset(&ime->comp, ime->comp.target_end);
  }
  event->result = ime->result_utf8;
  event->composite = ime->composite_utf8;
  event->cursor_pos = ime->cursor_pos;
  event->sel_start = ime->target_start;
  event->sel_end = ime->target_end;
}

#endif /* _WIN32 */
