/* Smoke tests for the BUI core (extracted blenlib subset).
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <math.h>
#include <stdio.h>
#include <string.h>

#include "BLI_dynstr.h"
#include "BLI_ghash.h"
#include "BLI_listbase.h"
#include "BLI_math_base.h"
#include "BLI_math_color.h"
#include "BLI_math_matrix.h"
#include "BLI_math_vector.h"
#include "BLI_memarena.h"
#include "BLI_mempool.h"
#include "BLI_string.h"
#include "BLI_string_utf8.h"
#include "MEM_guardedalloc.h"

static int failures = 0;

#define CHECK(cond) \
  do { \
    if (!(cond)) { \
      printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
      failures++; \
    } \
  } while (0)

static void test_listbase(void)
{
  ListBase lb = {NULL, NULL};
  LinkData a = {NULL, NULL}, c = {NULL, NULL};
  /* Heap-allocated so #BLI_freelinkN can free it. */
  LinkData *b = (LinkData *)MEM_callocN(sizeof(LinkData), "test b");
  BLI_addhead(&lb, &a);
  BLI_addtail(&lb, b);
  BLI_addtail(&lb, &c);
  CHECK(lb.first == &a);
  CHECK(lb.last == &c);
  CHECK(BLI_listbase_count(&lb) == 3);
  CHECK(BLI_findlink(&lb, 1) == b);
  BLI_freelinkN(&lb, b);
  CHECK(BLI_listbase_count(&lb) == 2);
  CHECK(((LinkData *)lb.first)->next == &c);
  BLI_pophead(&lb);
  BLI_poptail(&lb);
  CHECK(BLI_listbase_is_empty(&lb));
}

static void test_ghash(void)
{
  GHash *gh = BLI_ghash_str_new("test");
  BLI_ghash_insert(gh, (void *)"alpha", (void *)1);
  BLI_ghash_insert(gh, (void *)"beta", (void *)2);
  CHECK(BLI_ghash_len(gh) == 2);
  CHECK(BLI_ghash_lookup(gh, "beta") == (void *)2);
  CHECK(BLI_ghash_remove(gh, "alpha", NULL, NULL));
  CHECK(!BLI_ghash_haskey(gh, "alpha"));
  {
    /* Exercise the iterator API. */
    GHashIterator it;
    int count = 0;
    BLI_ghashIterator_init(&it, gh);
    while (!BLI_ghashIterator_done(&it)) {
      count++;
      BLI_ghashIterator_step(&it);
    }
    CHECK(count == 1);
  }
  BLI_ghash_free(gh, NULL, NULL);
}

static void test_dynstr(void)
{
  DynStr *ds = BLI_dynstr_new();
  BLI_dynstr_append(ds, "Hello, ");
  BLI_dynstr_append(ds, "BUI!");
  char *s = BLI_dynstr_get_cstring(ds);
  CHECK(strcmp(s, "Hello, BUI!") == 0);
  MEM_freeN(s);
  BLI_dynstr_free(ds);
}

static void test_mempool(void)
{
  BLI_mempool *pool = BLI_mempool_create(sizeof(int), 4, 64, 0);
  int *vals[10];
  for (int i = 0; i < 10; i++) {
    vals[i] = (int *)BLI_mempool_alloc(pool);
    *vals[i] = i * i;
  }
  CHECK(*vals[7] == 49);
  BLI_mempool_free(pool, vals[3]);
  int *recycled = (int *)BLI_mempool_alloc(pool);
  *recycled = 123;
  CHECK(recycled != NULL);
  BLI_mempool_destroy(pool);
}

static void test_memarena(void)
{
  MemArena *ma = BLI_memarena_new(1024, "test");
  char *p1 = (char *)BLI_memarena_alloc(ma, 100);
  memset(p1, 7, 100);
  char *p2 = (char *)BLI_memarena_calloc(ma, 50);
  for (int i = 0; i < 50; i++) {
    if (p2[i] != 0) {
      CHECK(false);
      break;
    }
  }
  BLI_memarena_free(ma);
}

static void test_math_vector_matrix(void)
{
  float v[3] = {3.0f, 4.0f, 0.0f};
  CHECK(fabsf(len_v3(v) - 5.0f) < 1e-6f);

  float m[4][4];
  unit_m4(m);
  float out[3];
  mul_v3_m4v3(out, m, v);
  CHECK(compare_v3v3(out, v, 1e-6f));

  float im[4][4], sm[4][4];
  const float src[4][4] = {
      {2, 0, 0, 1}, {0, 2, 0, 0}, {0, 0, 2, -1}, {0, 0, 0, 2}};
  copy_m4_m4(sm, src);
  CHECK(invert_m4_m4(im, sm));
  float ident[4][4];
  mul_m4_m4m4(ident, im, (const float(*)[4])src);
  for (int i = 0; i < 4; i++) {
    for (int j = 0; j < 4; j++) {
      CHECK(fabsf(ident[i][j] - ((i == j) ? 1.0f : 0.0f)) < 1e-4f);
    }
  }
}

static void test_color(void)
{
  float rgb[3] = {1.0f, 1.0f, 1.0f}, srgb[3];
  for (int i = 0; i < 3; i++) {
    srgb[i] = linearrgb_to_srgb(rgb[i]);
  }
  CHECK(fabsf(srgb[0] - 1.0f) < 1e-6f);
}

static void test_strings(void)
{
  CHECK(BLI_strlen_utf8("héllo") == 6);
  CHECK(BLI_strnlen("abc", 10) == 3);
  char buf[16];
  BLI_snprintf(buf, sizeof(buf), "%d-%s", 42, "x");
  CHECK(strcmp(buf, "42-x") == 0);
}

int main(void)
{
  test_listbase();
  test_ghash();
  test_dynstr();
  test_mempool();
  test_memarena();
  test_math_vector_matrix();
  test_color();
  test_strings();

  if (failures == 0) {
    printf("bui_core tests: ALL PASSED\n");
    return 0;
  }
  printf("bui_core tests: %d FAILURES\n", failures);
  return 1;
}
