/* Input methods on Win32: the window host's port of GHOST_ImeWin32 against a real window.
 *
 * The IME is off until a field is edited and on only while it is (BeginIME / EndIME), the
 * composition messages reach the kit, and a composition IMM32 can be driven to (it needs an
 * installed IME, so that part is skipped without one) arrives as drawn text and a result.
 * Exit 77 (skipped) off Windows.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#include <stdio.h>
#include <string.h>

#include "BUI_kit.h"

#ifndef _WIN32
int main(void)
{
  printf("not Windows: skipped\n");
  return 77;
}
#else

#  define WIN32_LEAN_AND_MEAN
#  include <windows.h>

#  include <imm.h>

static int g_fail = 0;
#  define CHECK(c) \
    do { \
      if (!(c)) { \
        printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #c); \
        g_fail = 1; \
      } \
    } while (0)

static void pump(void)
{
  for (int i = 0; i < 5; i++) {
    BK_window_pump();
    Sleep(5);
  }
}

static void click(HWND h, BKNode *n)
{
  int r[4];
  BK_node_rect(n, r);
  const LPARAM at = MAKELPARAM(r[0] + r[2] * 9 / 10, r[1] + r[3] / 2);
  SendMessageW(h, WM_LBUTTONDOWN, MK_LBUTTON, at);
  SendMessageW(h, WM_LBUTTONUP, 0, at);
  pump();
}

int main(void)
{
  if (!BK_window_supported()) {
    printf("no display: skipped\n");
    return 77;
  }
  BKNode *col = BK_node_new("column");
  BKNode *text = BK_node_new("text");
  BK_set_str(text, "value", "ab");
  BKNode *num = BK_node_new("number");
  BK_node_add(col, text, -1);
  BK_node_add(col, num, -1);
  BKContext *ctx = BK_context_new(col);
  BKWindow *win = BK_window_new(ctx, "BUI IME", 320, 200);
  if (!win) {
    printf("window failed\n");
    return 1;
  }
  BK_window_show(win);
  pump();
  BK_window_render_now(win);
  HWND h = (HWND)BK_window_handle(win);

  /* editing a text field turns the IME on (WM_window_IME_begin), ending the edit turns it off */
  click(h, text);
  CHECK(ImmGetContext(h) != NULL);
  SendMessageW(h, WM_KEYDOWN, VK_ESCAPE, 0);
  pump();
  CHECK(ImmGetContext(h) == NULL);

  /* number fields take no IME */
  click(h, num);
  CHECK(ImmGetContext(h) == NULL);
  SendMessageW(h, WM_KEYDOWN, VK_ESCAPE, 0);
  pump();

  /* the composition messages reach the kit: start composing, end composing */
  click(h, text);
  SendMessageW(h, WM_IME_STARTCOMPOSITION, 0, 0);
  CHECK(BK_ime_composing(ctx) == 1);
  SendMessageW(h, WM_IME_ENDCOMPOSITION, 0, 0);
  CHECK(BK_ime_composing(ctx) == 0);

  /* with an IME installed, drive a real composition through IMM32 and read it back */
  HIMC imc = ImmGetContext(h);
  if (imc) {
    const wchar_t comp[] = L"\x306B\x307B"; /* "にほ" */
    ImmSetOpenStatus(imc, TRUE);
    SendMessageW(h, WM_IME_STARTCOMPOSITION, 0, 0);
    if (ImmSetCompositionStringW(imc, SCS_SETSTR, (LPVOID)comp, sizeof(comp) - sizeof(wchar_t), NULL,
                                 0))
    {
      pump();
      BK_window_render_now(win);
      CHECK(BK_ime_composing(ctx) == 1);
      ImmNotifyIME(imc, NI_COMPOSITIONSTR, CPS_COMPLETE, 0);
      pump();
      printf("composition through IMM32: ok, value \"%s\"\n", BK_get_str(text, "value"));
    }
    else {
      printf("no IME to drive: composition through IMM32 skipped\n");
    }
    ImmReleaseContext(h, imc);
    SendMessageW(h, WM_IME_ENDCOMPOSITION, 0, 0);
  }
  SendMessageW(h, WM_KEYDOWN, VK_ESCAPE, 0);
  pump();

  BK_window_free(win);
  BK_context_free(ctx);
  if (!g_fail) {
    printf("ime win32 ok\n");
  }
  return g_fail;
}

#endif
