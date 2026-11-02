/* SPDX-FileCopyrightText: 2001-2002 NaN Holding BV. All rights reserved.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/** ile
 * \ingroup DNA
 * rief These structs are the foundation for all linked lists in the library system.
 *
 * Doubly-linked lists start from a ListBase and contain elements beginning
 * with Link.
 */

#pragma once

/* The de-C++ified core (core/include/DNA_listBase.h, shared through the
 * BUI_DNA_LISTBASE_SHARED guard) owns the global struct definitions. The
 * blender-qualified names below alias onto them so DNA structs and the C
 * listbase API share exactly one type. */
#ifndef BUI_DNA_LISTBASE_SHARED
#define BUI_DNA_LISTBASE_SHARED
typedef struct Link Link;
typedef struct LinkData LinkData;
typedef struct ListBase ListBase;
struct Link {
  struct Link *next, *prev;
};
struct LinkData {
  struct LinkData *next, *prev;
  void *data;
};
struct ListBase {
  void *first, *last;
};
#endif

#include "BLI_listbase_iterator.hh"

namespace blender {

using Link = ::Link;
using LinkData = ::LinkData;
using ListBase = ::ListBase;

/**
 * This is a thin wrapper around #ListBase to make it type-safe. It's designed to be used in DNA
 * structs. It is written as untyped #ListBase in .blend files for compatibility.
 */
template<typename T> struct ListBaseT : public ListBase {
  ListBaseTIterator<T> begin() const
  {
    return ListBaseTIterator<T>{static_cast<T *>(this->first)};
  }

  ListBaseTIterator<T> end() const
  {
    /* Don't use `this->last` because this iterator has to point to one-past-the-end. */
    return ListBaseTIterator<T>{nullptr};
  }

  /** Iterator that returns also gives an index for every item. This helps prevent mistakes
   * where continue mistakenly skips the incrementation.
   *
   * Usage: `for(auto [index, item] : list.enumerate())` */
  ListBaseEnumerateWrapper<T> enumerate()
  {
    return {this->first};
  }

  ListBaseEnumerateWrapper<const T> enumerate() const
  {
    return {this->first};
  }

  /** Iterator that supports removing the item we're looping over. */
  ListBaseMutableWrapper<T> items_mutable()
  {
    return {this->first};
  }

  /** Iterator that runs in reverse order. */
  ListBaseBackwardWrapper<T> items_reversed()
  {
    return {this->last};
  }

  ListBaseBackwardWrapper<const T> items_reversed() const
  {
    return {this->last};
  }

  /** Iterator that runs in reverse order and supports removing the item we're looping over. */
  ListBaseMutableBackwardWrapper<T> items_reversed_mutable()
  {
    return {this->last};
  }

  /* Cast for opaque types and C style subclasses. */
  template<typename OtherT> const ListBaseT<OtherT> &cast() const
  {
    return *reinterpret_cast<const ListBaseT<OtherT> *>(this);
  }

  template<typename OtherT> ListBaseT<OtherT> &cast()
  {
    return *reinterpret_cast<ListBaseT<OtherT> *>(this);
  }
};

}  // namespace blender
