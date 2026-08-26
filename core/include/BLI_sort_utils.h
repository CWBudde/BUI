/* SPDX-FileCopyrightText: 2013 Blender Authors
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

#pragma once


/** \file
 * \ingroup bli
 */

/**
 * \note keep \a sort_value first,
 * so cmp functions can be reused.
 */
typedef struct SortPtrByFloat SortPtrByFloat;
struct SortPtrByFloat {
  float sort_value;
  void *data;
};

typedef struct SortIntByFloat SortIntByFloat;
struct SortIntByFloat {
  float sort_value;
  int data;
};

typedef struct SortPtrByInt SortPtrByInt;
struct SortPtrByInt {
  int sort_value;
  void *data;
};

typedef struct SortIntByInt SortIntByInt;
struct SortIntByInt {
  int sort_value;
  int data;
};

int BLI_sortutil_cmp_float(const void *a_, const void *b_);
int BLI_sortutil_cmp_float_reverse(const void *a_, const void *b_);

int BLI_sortutil_cmp_int(const void *a_, const void *b_);
int BLI_sortutil_cmp_int_reverse(const void *a_, const void *b_);




