/* SPDX-FileCopyrightText: 2001-2002 NaN Holding BV. All rights reserved.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */

/** \file
 * \ingroup DNA
 */

#pragma once


/* types */

/** vector of two shorts. */
typedef struct vec2s vec2s;
struct vec2s {
  short x, y;
};

/** vector of two floats. */
typedef struct vec2f vec2f;
struct vec2f {
  float x, y;
};

typedef struct vec2i vec2i;
struct vec2i {
  int x, y;
};

/* not used at the moment */
/*
typedef struct vec2d vec2d;
struct vec2d {
  double x, y;
};
*/
typedef struct vec3i vec3i;
struct vec3i {
  int x, y, z;
};

typedef struct vec3f vec3f;
struct vec3f {
  float x, y, z;
};
/*
typedef struct vec3d vec3d;
struct vec3d {
  double x, y, z;
};

typedef struct vec4i vec4i;
struct vec4i {
  int x, y, z, w;
};
*/
typedef struct vec4f vec4f;
struct vec4f {
  float x, y, z, w;
};

/**
 * This type generally shouldn't be used. It only exists for cases where a DNA type that
 * corresponds to `blender:float4x4` is required. Note that `float4x4` is 16 byte aligned, but we
 * can't enforce that this struct yet.
 */
typedef struct mat4x4f mat4x4f;
struct mat4x4f {
  float value[4][4];
};
/*
typedef struct vec4d vec4d;
struct vec4d {
  double x, y, z, w;
};
*/

/** integer rectangle. */
typedef struct rcti rcti;
struct rcti {
  int xmin, xmax;
  int ymin, ymax;

#ifdef __cplusplus
  bool operator==(const rcti &other) const
  {
    return xmin == other.xmin && xmax == other.xmax && ymin == other.ymin && ymax == other.ymax;
  }
  bool operator!=(const rcti &other) const
  {
    return !(*this == other);
  }
#endif
};

/** float rectangle. */
typedef struct rctf rctf;
struct rctf {
  float xmin, xmax;
  float ymin, ymax;
};

/** dual quaternion. */
typedef struct DualQuat DualQuat;
struct DualQuat {
  float quat[4];
  float trans[4];

  float scale[4][4];
  float scale_weight;
};





