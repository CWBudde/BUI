/* SPDX-FileCopyrightText: 2013 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/** \file
 * \ingroup bli
 *
 * Utility functions for sorting common types.
 */

#include "BLI_sort_utils.h" /* own include */

typedef struct SortAnyByFloat SortAnyByFloat;
struct SortAnyByFloat {
  float sort_value;
};

typedef struct SortAnyByInt SortAnyByInt;
struct SortAnyByInt {
  int sort_value;
};

typedef struct SortAnyByPtr SortAnyByPtr;
struct SortAnyByPtr {
  const void *sort_value;
};

int BLI_sortutil_cmp_float(const void *a_, const void *b_)
{
  const SortAnyByFloat *a = (const SortAnyByFloat *)(a_);
  const SortAnyByFloat *b = (const SortAnyByFloat *)(b_);
  if (a->sort_value > b->sort_value) {
    return 1;
  }
  if (a->sort_value < b->sort_value) {
    return -1;
  }

  return 0;
}

int BLI_sortutil_cmp_float_reverse(const void *a_, const void *b_)
{
  const SortAnyByFloat *a = (const SortAnyByFloat *)(a_);
  const SortAnyByFloat *b = (const SortAnyByFloat *)(b_);
  if (a->sort_value < b->sort_value) {
    return 1;
  }
  if (a->sort_value > b->sort_value) {
    return -1;
  }

  return 0;
}

int BLI_sortutil_cmp_int(const void *a_, const void *b_)
{
  const SortAnyByInt *a = (const SortAnyByInt *)(a_);
  const SortAnyByInt *b = (const SortAnyByInt *)(b_);
  if (a->sort_value > b->sort_value) {
    return 1;
  }
  if (a->sort_value < b->sort_value) {
    return -1;
  }

  return 0;
}

int BLI_sortutil_cmp_int_reverse(const void *a_, const void *b_)
{
  const SortAnyByInt *a = (const SortAnyByInt *)(a_);
  const SortAnyByInt *b = (const SortAnyByInt *)(b_);
  if (a->sort_value < b->sort_value) {
    return 1;
  }
  if (a->sort_value > b->sort_value) {
    return -1;
  }

  return 0;
}




