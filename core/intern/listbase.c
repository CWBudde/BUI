/* SPDX-FileCopyrightText: 2001-2002 NaN Holding BV. All rights reserved.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/** \file
 * \ingroup bli
 *
 * Manipulations on double-linked list (#ListBase structs).
 *
 * For single linked lists see `BLI_linklist.h`.
 */

#include <stdlib.h>
#include <string.h>

#include "MEM_guardedalloc.h"

#include "DNA_listBase.h"

#include "BLI_assert.h"
#include "BLI_listbase.h"

#include "BLI_strict_flags.h" /* IWYU pragma: keep. Keep last. */

#define LISTBASE_FOREACH(type, var, list) \
  for (type var = (type)((list)->first); var != NULL; var = (type)(((Link *)(var))->next))
#define LISTBASE_FOREACH_BACKWARD(type, var, list) \
  for (type var = (type)((list)->last); var != NULL; var = (type)(((Link *)(var))->prev))

void BLI_movelisttolist(ListBase *dst, ListBase *src)
{
  if (src->first == NULL) {
    return;
  }

  if (dst->first == NULL) {
    dst->first = src->first;
    dst->last = src->last;
  }
  else {
    ((Link *)(dst->last))->next = (Link *)(src->first);
    ((Link *)(src->first))->prev = (Link *)(dst->last);
    dst->last = src->last;
  }
  src->first = src->last = NULL;
}

void BLI_movelisttolist_reverse(ListBase *dst, ListBase *src)
{
  if (src->first == NULL) {
    return;
  }

  if (dst->first == NULL) {
    dst->first = src->first;
    dst->last = src->last;
  }
  else {
    ((Link *)(src->last))->next = (Link *)(dst->first);
    ((Link *)(dst->first))->prev = (Link *)(src->last);
    dst->first = src->first;
  }

  src->first = src->last = NULL;
}

void BLI_listbase_split_after(ListBase *original_listbase, ListBase *split_listbase, void *vlink)
{
  BLI_assert(BLI_listbase_is_empty(split_listbase));
  BLI_assert(vlink == NULL || BLI_findindex(original_listbase, vlink) >= 0);

  if (vlink == original_listbase->last) {
    /* Nothing to split, and `split_listbase` is assumed already empty (see assert above). */
    return;
  }

  if (vlink == NULL) {
    /* Move everything into `split_listbase`. */
    BUI_SWAP(*original_listbase, *split_listbase);
    return;
  }

  Link *link = (Link *)(vlink);
  Link *next_link = link->next;
  BLI_assert(next_link != NULL);
  Link *last_link = (Link *)(original_listbase->last);

  original_listbase->last = link;
  split_listbase->first = next_link;
  split_listbase->last = last_link;

  link->next = NULL;
  next_link->prev = NULL;
}

void BLI_addhead(ListBase *listbase, void *vlink)
{
  Link *link = (Link *)(vlink);

  if (link == NULL) {
    return;
  }

  link->next = (Link *)(listbase->first);
  link->prev = NULL;

  if (listbase->first) {
    ((Link *)(listbase->first))->prev = link;
  }
  if (listbase->last == NULL) {
    listbase->last = link;
  }
  listbase->first = link;
}

void BLI_addtail(ListBase *listbase, void *vlink)
{
  Link *link = (Link *)(vlink);

  if (link == NULL) {
    return;
  }

  link->next = NULL;
  link->prev = (Link *)(listbase->last);

  if (listbase->last) {
    ((Link *)(listbase->last))->next = link;
  }
  if (listbase->first == NULL) {
    listbase->first = link;
  }
  listbase->last = link;
}

void BLI_remlink(ListBase *listbase, void *vlink)
{
  Link *link = (Link *)(vlink);

  if (link == NULL) {
    return;
  }

  if (link->next) {
    link->next->prev = link->prev;
  }
  if (link->prev) {
    link->prev->next = link->next;
  }

  if (listbase->last == link) {
    listbase->last = link->prev;
  }
  if (listbase->first == link) {
    listbase->first = link->next;
  }
}

bool BLI_remlink_safe(ListBase *listbase, void *vlink)
{
  if (BLI_findindex(listbase, vlink) != -1) {
    BLI_remlink(listbase, vlink);
    return true;
  }

  return false;
}

void BLI_listbase_swaplinks(ListBase *listbase, void *vlinka, void *vlinkb)
{
  Link *linka = (Link *)(vlinka);
  Link *linkb = (Link *)(vlinkb);

  if (!linka || !linkb) {
    return;
  }

  if (linkb->next == linka) {
    BUI_SWAP(linka, linkb);
  }

  if (linka->next == linkb) {
    linka->next = linkb->next;
    linkb->prev = linka->prev;
    linka->prev = linkb;
    linkb->next = linka;
  }
  else { /* Non-contiguous items, we can safely swap. */
    BUI_SWAP(linka->prev, linkb->prev);
    BUI_SWAP(linka->next, linkb->next);
  }

  /* Update neighbors of linka and linkb. */
  if (linka->prev) {
    linka->prev->next = linka;
  }
  if (linka->next) {
    linka->next->prev = linka;
  }
  if (linkb->prev) {
    linkb->prev->next = linkb;
  }
  if (linkb->next) {
    linkb->next->prev = linkb;
  }

  if (listbase->last == linka) {
    listbase->last = linkb;
  }
  else if (listbase->last == linkb) {
    listbase->last = linka;
  }

  if (listbase->first == linka) {
    listbase->first = linkb;
  }
  else if (listbase->first == linkb) {
    listbase->first = linka;
  }
}

void BLI_listbases_swaplinks(ListBase *listbasea, ListBase *listbaseb, void *vlinka, void *vlinkb)
{
  Link *linka = (Link *)(vlinka);
  Link *linkb = (Link *)(vlinkb);
  Link linkc = {NULL};

  if (!linka || !linkb) {
    return;
  }

  /* The reference to `linkc` assigns NULL, not a dangling pointer so it can be ignored. */
#if defined(__GNUC__) && (__GNUC__ * 100 + __GNUC_MINOR__) >= 1201 /* gcc12.1+ only */
#  pragma GCC diagnostic push
#  pragma GCC diagnostic ignored "-Wdangling-pointer"
#endif

  /* Temporary link to use as placeholder of the links positions */
  BLI_insertlinkafter(listbasea, linka, &linkc);

#if defined(__GNUC__) && (__GNUC__ * 100 + __GNUC_MINOR__) >= 1201 /* gcc12.1+ only */
#  pragma GCC diagnostic pop
#endif

  /* Bring linka into linkb position */
  BLI_remlink(listbasea, linka);
  BLI_insertlinkafter(listbaseb, linkb, linka);

  /* Bring linkb into linka position */
  BLI_remlink(listbaseb, linkb);
  BLI_insertlinkafter(listbasea, &linkc, linkb);

  /* Remove temporary link */
  BLI_remlink(listbasea, &linkc);
}

void *BLI_pophead(ListBase *listbase)
{
  Link *link = (Link *)(listbase->first);
  if (link) {
    BLI_remlink(listbase, link);
  }
  return link;
}

void *BLI_poptail(ListBase *listbase)
{
  Link *link = (Link *)(listbase->last);
  if (link) {
    BLI_remlink(listbase, link);
  }
  return link;
}

void BLI_freelinkN(ListBase *listbase, void *vlink)
{
  Link *link = (Link *)(vlink);

  if (link == NULL) {
    return;
  }

  BLI_remlink(listbase, link);
  MEM_freeN(link);
}

/**
 * Assigns all #Link.prev pointers from #Link.next
 */
static void listbase_double_from_single(Link *iter, ListBase *listbase)
{
  Link *prev = NULL;
  listbase->first = iter;
  do {
    iter->prev = prev;
    prev = iter;
  } while ((iter = iter->next));
  listbase->last = prev;
}

#define SORT_IMPL_LINKTYPE Link

/* regular call */
#define SORT_IMPL_FUNC listbase_sort_fn
#include "list_sort_impl.h"
#undef SORT_IMPL_FUNC

/* re-entrant call */
#define SORT_IMPL_USE_THUNK
#define SORT_IMPL_FUNC listbase_sort_fn_r
#include "list_sort_impl.h"
#undef SORT_IMPL_FUNC
#undef SORT_IMPL_USE_THUNK

#undef SORT_IMPL_LINKTYPE

void BLI_listbase_sort(ListBase *listbase, int (*cmp)(const void *, const void *))
{
  if (listbase->first != listbase->last) {
    Link *head = (Link *)(listbase->first);
    head = listbase_sort_fn(head, cmp);
    listbase_double_from_single(head, listbase);
  }
}

void BLI_listbase_sort_r(ListBase *listbase,
                         int (*cmp)(void *, const void *, const void *),
                         void *thunk)
{
  if (listbase->first != listbase->last) {
    Link *head = (Link *)(listbase->first);
    head = listbase_sort_fn_r(head, cmp, thunk);
    listbase_double_from_single(head, listbase);
  }
}

void BLI_insertlinkafter(ListBase *listbase, void *vprevlink, void *vnewlink)
{
  Link *prevlink = (Link *)(vprevlink);
  Link *newlink = (Link *)(vnewlink);

  /* newlink before nextlink */
  if (newlink == NULL) {
    return;
  }

  /* empty list */
  if (listbase->first == NULL) {
    listbase->first = newlink;
    listbase->last = newlink;
    return;
  }

  /* insert at head of list */
  if (prevlink == NULL) {
    newlink->prev = NULL;
    newlink->next = (Link *)(listbase->first);
    newlink->next->prev = newlink;
    listbase->first = newlink;
    return;
  }

  /* at end of list */
  if (listbase->last == prevlink) {
    listbase->last = newlink;
  }

  newlink->next = prevlink->next;
  newlink->prev = prevlink;
  prevlink->next = newlink;
  if (newlink->next) {
    newlink->next->prev = newlink;
  }
}

void BLI_insertlinkbefore(ListBase *listbase, void *vnextlink, void *vnewlink)
{
  Link *nextlink = (Link *)(vnextlink);
  Link *newlink = (Link *)(vnewlink);

  /* newlink before nextlink */
  if (newlink == NULL) {
    return;
  }

  /* empty list */
  if (listbase->first == NULL) {
    listbase->first = newlink;
    listbase->last = newlink;
    return;
  }

  /* insert at end of list */
  if (nextlink == NULL) {
    newlink->prev = (Link *)(listbase->last);
    newlink->next = NULL;
    ((Link *)(listbase->last))->next = newlink;
    listbase->last = newlink;
    return;
  }

  /* at beginning of list */
  if (listbase->first == nextlink) {
    listbase->first = newlink;
  }

  newlink->next = nextlink;
  newlink->prev = nextlink->prev;
  nextlink->prev = newlink;
  if (newlink->prev) {
    newlink->prev->next = newlink;
  }
}

void BLI_insertlinkreplace(ListBase *listbase, void *vreplacelink, void *vnewlink)
{
  Link *l_old = (Link *)(vreplacelink);
  Link *l_new = (Link *)(vnewlink);

  /* update adjacent links */
  if (l_old->next != NULL) {
    l_old->next->prev = l_new;
  }
  if (l_old->prev != NULL) {
    l_old->prev->next = l_new;
  }

  /* set direct links */
  l_new->next = l_old->next;
  l_new->prev = l_old->prev;

  /* update list */
  if (listbase->first == l_old) {
    listbase->first = l_new;
  }
  if (listbase->last == l_old) {
    listbase->last = l_new;
  }
}

bool BLI_listbase_link_move(ListBase *listbase, void *vlink, int step)
{
  Link *link = (Link *)(vlink);
  Link *hook = link;
  const bool is_up = step < 0;

  if (step == 0) {
    return false;
  }
  BLI_assert(BLI_findindex(listbase, link) != -1);

  /* find link to insert before/after */
  const int abs_step = abs(step);
  for (int i = 0; i < abs_step; i++) {
    hook = is_up ? hook->prev : hook->next;
    if (!hook) {
      return false;
    }
  }

  /* reinsert link */
  BLI_remlink(listbase, vlink);
  if (is_up) {
    BLI_insertlinkbefore(listbase, hook, vlink);
  }
  else {
    BLI_insertlinkafter(listbase, hook, vlink);
  }
  return true;
}

bool BLI_listbase_move_index(ListBase *listbase, int from, int to)
{
  if (from == to) {
    return false;
  }

  /* Find the link to move. */
  void *link = BLI_findlink(listbase, from);

  if (!link) {
    return false;
  }

  return BLI_listbase_link_move(listbase, link, to - from);
}

void BLI_freelist(ListBase *listbase)
{
  Link *link, *next;

  link = (Link *)(listbase->first);
  while (link) {
    next = link->next;
    free(link);
    link = next;
  }

  BLI_listbase_clear(listbase);
}

void BLI_freelistN(ListBase *listbase)
{
  Link *link, *next;

  link = (Link *)(listbase->first);
  while (link) {
    next = link->next;
    MEM_freeN(link);
    link = next;
  }

  BLI_listbase_clear(listbase);
}

int BLI_listbase_count_at_most(const ListBase *listbase, const int count_max)
{
  Link *link;
  int count = 0;

  for (link = (Link *)(listbase->first); link && count != count_max; link = link->next)
  {
    count++;
  }

  return count;
}

int BLI_listbase_count(const ListBase *listbase)
{
  int count = 0;
  LISTBASE_FOREACH (Link *, link, listbase) {
    count++;
  }

  return count;
}

void *BLI_findlink(const ListBase *listbase, int number)
{
  Link *link = NULL;

  if (number >= 0) {
    link = (Link *)(listbase->first);
    while (link != NULL && number != 0) {
      number--;
      link = link->next;
    }
  }

  return link;
}

void *BLI_rfindlink(const ListBase *listbase, int number)
{
  Link *link = NULL;

  if (number >= 0) {
    link = (Link *)(listbase->last);
    while (link != NULL && number != 0) {
      number--;
      link = link->prev;
    }
  }

  return link;
}

void *BLI_findlinkfrom(Link *start, int step)
{
  Link *link = NULL;

  if (step >= 0) {
    link = start;
    while (link != NULL && step != 0) {
      step--;
      link = link->next;
    }
  }
  else {
    link = start;
    while (link != NULL && step != 0) {
      step++;
      link = link->prev;
    }
  }

  return link;
}

int BLI_findindex(const ListBase *listbase, const void *vlink)
{
  Link *link = NULL;
  int number = 0;

  if (vlink == NULL) {
    return -1;
  }

  link = (Link *)(listbase->first);
  while (link) {
    if (link == vlink) {
      return number;
    }

    number++;
    link = link->next;
  }

  return -1;
}

void *BLI_findstring(const ListBase *listbase, const char *id, const int offset)
{
  const char *id_iter;

  if (id == NULL) {
    return NULL;
  }

  LISTBASE_FOREACH (Link *, link, listbase) {
    id_iter = ((const char *)(link)) + offset;

    if (id[0] == id_iter[0] && STREQ(id, id_iter)) {
      return link;
    }
  }

  return NULL;
}
void *BLI_rfindstring(const ListBase *listbase, const char *id, const int offset)
{
  /* Same as #BLI_findstring but find reverse. */
  LISTBASE_FOREACH_BACKWARD (Link *, link, listbase) {
    const char *id_iter = ((const char *)(link)) + offset;
    if (id[0] == id_iter[0] && STREQ(id, id_iter)) {
      return link;
    }
  }

  return NULL;
}
void *BLI_listbase_findafter_string(Link *link, const char *id, const int offset)
{
  /* Same as #BLI_findstring but start searching after given link. */
  for (link = link->next; link; link = link->next) {
    const char *id_iter = ((const char *)(link)) + offset;
    if (id[0] == id_iter[0] && STREQ(id, id_iter)) {
      return link;
    }
  }

  return NULL;
}

void *BLI_findstring_ptr(const ListBase *listbase, const char *id, const int offset)
{
  const char *id_iter;

  LISTBASE_FOREACH (Link *, link, listbase) {
    /* Exact copy of BLI_findstring(), except for this line, and the check for potential NULL
     * below. */
    id_iter = *((const char **)(
        (char *)((((const char *)(link)) + offset))));
    if (id_iter && id[0] == id_iter[0] && STREQ(id, id_iter)) {
      return link;
    }
  }

  return NULL;
}
void *BLI_rfindstring_ptr(const ListBase *listbase, const char *id, const int offset)
{
  /* Same as #BLI_findstring_ptr but find reverse. */

  const char *id_iter;

  LISTBASE_FOREACH_BACKWARD (Link *, link, listbase) {
    /* Exact copy of BLI_rfindstring(), except for this line, and the check for potential NULL
     * below. */
    id_iter = *((const char **)(
        (char *)((((const char *)(link)) + offset))));
    if (id_iter && id[0] == id_iter[0] && STREQ(id, id_iter)) {
      return link;
    }
  }

  return NULL;
}

void *BLI_listbase_findafter_string_ptr(Link *link, const char *id, const int offset)
{
  const char *id_iter;

  for (link = link->next; link; link = link->next) {
    /* Exact copy of BLI_findstring(), except for this line, and the check for potential NULL
     * below. */
    id_iter = *((const char **)(
        (char *)((((const char *)(link)) + offset))));
    if (id_iter && id[0] == id_iter[0] && STREQ(id, id_iter)) {
      return link;
    }
  }

  return NULL;
}

void *BLI_findptr(const ListBase *listbase, const void *ptr, const int offset)
{
  LISTBASE_FOREACH (Link *, link, listbase) {
    /* Exact copy of #BLI_findstring(), except for this line. */
    const void *ptr_iter = *((const void **)(
        (char *)((((const char *)(link)) + offset))));
    if (ptr == ptr_iter) {
      return link;
    }
  }
  return NULL;
}
void *BLI_rfindptr(const ListBase *listbase, const void *ptr, const int offset)
{
  /* Same as #BLI_findptr but find reverse. */
  LISTBASE_FOREACH_BACKWARD (Link *, link, listbase) {
    /* Exact copy of #BLI_rfindstring(), except for this line. */
    const void *ptr_iter = *((const void **)(
        (char *)((((const char *)(link)) + offset))));

    if (ptr == ptr_iter) {
      return link;
    }
  }
  return NULL;
}

void *BLI_listbase_bytes_find(const ListBase *listbase,
                              const void *bytes,
                              const size_t bytes_size,
                              const int offset)
{
  LISTBASE_FOREACH (Link *, link, listbase) {
    const void *ptr_iter = (const void *)(((const char *)(link)) +
                                                     offset);
    if (memcmp(bytes, ptr_iter, bytes_size) == 0) {
      return link;
    }
  }

  return NULL;
}
void *BLI_listbase_bytes_rfind(const ListBase *listbase,
                               const void *bytes,
                               const size_t bytes_size,
                               const int offset)
{
  /* Same as #BLI_listbase_bytes_find but find reverse. */
  LISTBASE_FOREACH_BACKWARD (Link *, link, listbase) {
    const void *ptr_iter = (const void *)(((const char *)(link)) +
                                                     offset);
    if (memcmp(bytes, ptr_iter, bytes_size) == 0) {
      return link;
    }
  }
  return NULL;
}

void *BLI_listbase_string_or_index_find(const ListBase *listbase,
                                        const char *string,
                                        const size_t string_offset,
                                        const int index)
{
  Link *link = NULL;
  Link *link_at_index = NULL;

  int index_iter;
  for (link = (Link *)(listbase->first), index_iter = 0; link;
       link = link->next, index_iter++)
  {
    if (string != NULL && string[0] != '\0') {
      const char *string_iter = ((const char *)(link)) + string_offset;

      if (string[0] == string_iter[0] && STREQ(string, string_iter)) {
        return link;
      }
    }
    if (index_iter == index) {
      link_at_index = link;
    }
  }
  return link_at_index;
}

int BLI_findstringindex(const ListBase *listbase, const char *id, const int offset)
{
  Link *link = NULL;
  const char *id_iter;
  int i = 0;

  link = (Link *)(listbase->first);
  while (link) {
    id_iter = ((const char *)(link)) + offset;

    if (id[0] == id_iter[0] && STREQ(id, id_iter)) {
      return i;
    }
    i++;
    link = link->next;
  }

  return -1;
}

ListBase BLI_listbase_from_link(Link *some_link)
{
  ListBase list = {some_link, some_link};
  if (some_link == NULL) {
    return list;
  }

  /* Find the first element. */
  while (((Link *)(list.first))->prev != NULL) {
    list.first = ((Link *)(list.first))->prev;
  }

  /* Find the last element. */
  while (((Link *)(list.last))->next != NULL) {
    list.last = ((Link *)(list.last))->next;
  }

  return list;
}

void BLI_duplicatelist(ListBase *dst, const ListBase *src)
{
  Link *dst_link, *src_link;

  /* in this order, to ensure it works if dst == src */
  src_link = (Link *)(src->first);
  dst->first = dst->last = NULL;

  while (src_link) {
    dst_link = MEM_dupallocN(src_link);
    BLI_addtail(dst, dst_link);

    src_link = src_link->next;
  }
}

void BLI_listbase_reverse(ListBase *lb)
{
  Link *curr = (Link *)(lb->first);
  Link *prev = NULL;
  Link *next = NULL;
  while (curr) {
    next = curr->next;
    curr->next = prev;
    curr->prev = next;
    prev = curr;
    curr = next;
  }

  /* swap first/last */
  curr = (Link *)(lb->first);
  lb->first = lb->last;
  lb->last = curr;
}

void BLI_listbase_rotate_first(ListBase *lb, void *vlink)
{
  /* make circular */
  ((Link *)(lb->first))->prev = (Link *)(lb->last);
  ((Link *)(lb->last))->next = (Link *)(lb->first);

  lb->first = vlink;
  lb->last = ((Link *)(vlink))->prev;

  ((Link *)(lb->first))->prev = NULL;
  ((Link *)(lb->last))->next = NULL;
}

void BLI_listbase_rotate_last(ListBase *lb, void *vlink)
{
  /* make circular */
  ((Link *)(lb->first))->prev = (Link *)(lb->last);
  ((Link *)(lb->last))->next = (Link *)(lb->first);

  lb->first = ((Link *)(vlink))->next;
  lb->last = vlink;

  ((Link *)(lb->first))->prev = NULL;
  ((Link *)(lb->last))->next = NULL;
}

bool BLI_listbase_validate(ListBase *lb)
{
  if (lb->first == NULL && lb->last == NULL) {
    /* Empty list. */
    return true;
  }
  if (ELEM(NULL, lb->first, lb->last)) {
    /* If one of the pointer is null, but not this other, this is a corrupted listbase. */
    return false;
  }

  /* Walk the list in both directions to ensure all next & prev pointers are valid and consistent.
   */
  LISTBASE_FOREACH (Link *, lb_link, lb) {
    if (lb_link == lb->first) {
      if (lb_link->prev != NULL) {
        return false;
      }
    }
    if (lb_link == lb->last) {
      if (lb_link->next != NULL) {
        return false;
      }
    }
  }
  LISTBASE_FOREACH_BACKWARD (Link *, lb_link, lb) {
    if (lb_link == lb->last) {
      if (lb_link->next != NULL) {
        return false;
      }
    }
    if (lb_link == lb->first) {
      if (lb_link->prev != NULL) {
        return false;
      }
    }
  }

  return true;
}

LinkData *BLI_genericNodeN(void *data)
{
  LinkData *ld;

  if (data == NULL) {
    return NULL;
  }

  /* create new link, and make it hold the given data */
  ld = ((LinkData *)MEM_callocN(sizeof(LinkData), __func__));
  ld->data = data;

  return ld;
}








