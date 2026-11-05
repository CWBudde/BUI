/* SPDX-FileCopyrightText: 2020 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/** \file
 * \ingroup edinterface
 *
 * Undo stack to use for UI widgets that manage their own editing state.
 */

/* BUI: copied from editors/interface/interface_undo.cc: the undo stack of an
 * active text edit (Ctrl+Z / Ctrl+Shift+Z while typing, kit_core.c bk_te_key).
 * BUI edits, marked "BUI:": the states list is linked by hand (the kit's core
 * ListBase API is C, ListBaseT is not vendored), states are plain malloc
 * blocks, and C entry points are added at the end (bk_blender.h). */

#include <cstdlib>
#include <cstring>

#include "bk_blender.h"

namespace blender::ui {

/* -------------------------------------------------------------------- */
/** \name Text Field Undo Stack
 * \{ */

struct UndoStack_Text_State {
  UndoStack_Text_State *next, *prev;
  int cursor_index;
  char text[1]; /* BUI: [1] (C++ has no zero-length arrays) */
};

/* BUI: a first/last pair instead of ListBaseT<UndoStack_Text_State>. */
struct UndoStack_Text_States {
  UndoStack_Text_State *first, *last;
};

struct UndoStack_Text {
  UndoStack_Text_States states;
  UndoStack_Text_State *current;
};

/* BUI: BLI_remlink / BLI_addtail for this list. */
static void states_remlink(UndoStack_Text_States *lb, UndoStack_Text_State *state)
{
  if (state->next) {
    state->next->prev = state->prev;
  }
  if (state->prev) {
    state->prev->next = state->next;
  }
  if (lb->last == state) {
    lb->last = state->prev;
  }
  if (lb->first == state) {
    lb->first = state->next;
  }
}

static void states_addtail(UndoStack_Text_States *lb, UndoStack_Text_State *state)
{
  state->next = nullptr;
  state->prev = lb->last;
  if (lb->last) {
    lb->last->next = state;
  }
  if (lb->first == nullptr) {
    lb->first = state;
  }
  lb->last = state;
}

static const char *textedit_undo_impl(UndoStack_Text *stack, int *r_cursor_index)
{
  /* Don't undo if no data has been pushed yet. */
  if (stack->current == nullptr) {
    return nullptr;
  }

  /* Travel backwards in the stack and copy information to the caller. */
  if (stack->current->prev != nullptr) {
    stack->current = stack->current->prev;

    *r_cursor_index = stack->current->cursor_index;
    return stack->current->text;
  }
  return nullptr;
}

static const char *textedit_redo_impl(UndoStack_Text *stack, int *r_cursor_index)
{
  /* Don't redo if no data has been pushed yet. */
  if (stack->current == nullptr) {
    return nullptr;
  }

  /* Only redo if new data has not been entered since the last undo. */
  if (stack->current->next) {
    stack->current = stack->current->next;

    *r_cursor_index = stack->current->cursor_index;
    return stack->current->text;
  }
  return nullptr;
}

const char *textedit_undo(UndoStack_Text *stack, int direction, int *r_cursor_index)
{
  if (direction < 0) {
    return textedit_undo_impl(stack, r_cursor_index);
  }
  return textedit_redo_impl(stack, r_cursor_index);
}

void textedit_undo_push(UndoStack_Text *stack, const char *text, int cursor_index)
{
  /* Clear all redo actions from the current state. */
  if (stack->current != nullptr) {
    while (stack->current->next) {
      UndoStack_Text_State *state = stack->current->next;
      states_remlink(&stack->states, state);
      free(state); /* BUI: free (Blender: MEM_delete) */
    }
  }

  /* Create the new state. */
  const int text_size = int(strlen(text)) + 1;
  stack->current = static_cast<UndoStack_Text_State *>(
      malloc(sizeof(UndoStack_Text_State) + text_size)); /* BUI: malloc */
  stack->current->cursor_index = cursor_index;
  memcpy(stack->current->text, text, text_size);
  states_addtail(&stack->states, stack->current);
}

UndoStack_Text *textedit_undo_stack_create()
{
  UndoStack_Text *stack = static_cast<UndoStack_Text *>(calloc(1, sizeof(UndoStack_Text)));
  stack->current = nullptr;
  stack->states.first = stack->states.last = nullptr;

  return stack;
}

void textedit_undo_stack_destroy(UndoStack_Text *stack)
{
  /* BUI: BLI_freelistN */
  for (UndoStack_Text_State *state = stack->states.first; state;) {
    UndoStack_Text_State *next = state->next;
    free(state);
    state = next;
  }
  free(stack);
}

/** \} */

}  // namespace blender::ui

/* BUI: C entry points. */
BKTextUndo *bk_textedit_undo_stack_create(void)
{
  return reinterpret_cast<BKTextUndo *>(blender::ui::textedit_undo_stack_create());
}

void bk_textedit_undo_stack_destroy(BKTextUndo *stack)
{
  if (stack) {
    blender::ui::textedit_undo_stack_destroy(
        reinterpret_cast<blender::ui::UndoStack_Text *>(stack));
  }
}

void bk_textedit_undo_push(BKTextUndo *stack, const char *text, int cursor_index)
{
  blender::ui::textedit_undo_push(
      reinterpret_cast<blender::ui::UndoStack_Text *>(stack), text, cursor_index);
}

const char *bk_textedit_undo(BKTextUndo *stack, int direction, int *r_cursor_index)
{
  return blender::ui::textedit_undo(
      reinterpret_cast<blender::ui::UndoStack_Text *>(stack), direction, r_cursor_index);
}
