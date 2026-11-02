/* BUI seam: ID-name helpers used by search/templating drawing.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#pragma once

#include "DNA_ID.h"

#ifdef __cplusplus
extern "C" {
using blender::ID;
#endif

#ifndef MAX_ID_FULL_NAME_UI
#define MAX_ID_FULL_NAME_UI (MAX_NAME + 23)
#endif

/* Stub implementation: copies the ID's name (no library prefix decoration)
 * and reports zero prefix length. A real host can decorate linked-data names. */
void BKE_id_full_name_ui_prefix_get(char name[MAX_ID_FULL_NAME_UI],
                                    const ID *id,
                                    bool add_lib_hint,
                                    char separator_char,
                                    int *r_prefix_len);

#ifdef __cplusplus
}
#endif
