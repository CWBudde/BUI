// BUI GPU renderer shaders (Direct3D 11, shader model 4.0).
//
// One instanced quad per primitive, like Blender's GPU_SHADER_2D_WIDGET_BASE_INST
// batches (interface_widgets.cc): the CPU appends 144-byte instances and every
// run of them is a single DrawInstanced. The pixel shader evaluates the same
// coverage formulas as the software raster (gpu_raster.c), so both renderers
// produce the same image. Output is premultiplied (blend ONE, INV_SRC_ALPHA);
// layers inside one primitive (emboss, fill, outline) are composited here,
// in the order the software raster blends them.
//
// Rebuild gpu_d3d11_shaders.h after editing (see gpu/shaders/build.sh).
//
// SPDX-License-Identifier: GPL-2.0-or-later

#define K_RECT 0
#define K_WIDGET 1
#define K_ELLIPSE 2
#define K_LINE 3
#define K_MASK 4
#define K_IMAGE 5
#define K_SHADOW 6
#define K_GRADIENT 7
#define K_HSV 8
#define K_SPHERE 9

#define W_SHADE_H 1
#define W_CHECKER 2
#define W_LEGACY 4
#define W_INNER 8

cbuffer Frame : register(b0)
{
  float2 to_ndc; // 2 / size
  float2 pad_;
};

Texture2D<float> mask_tex : register(t0);
Texture2D<float4> rgba_tex : register(t1);

struct Inst {
  float4 rect : RECT;
  float4 bounds : BOUNDS; // pixel box x0, y0, x1, y1, already clipped
  float4 c0 : COLOR0;
  float4 c1 : COLOR1;
  float4 c2 : COLOR2;
  float4 c3 : COLOR3;
  float4 p : PARAMS; // x = kind
  float4 uv : EXTRA;
};

struct VOut {
  float4 pos : SV_Position;
  nointerpolation float4 rect : RECT;
  nointerpolation float4 c0 : COLOR0;
  nointerpolation float4 c1 : COLOR1;
  nointerpolation float4 c2 : COLOR2;
  nointerpolation float4 c3 : COLOR3;
  nointerpolation float4 p : PARAMS;
  nointerpolation float4 uv : EXTRA;
};

VOut vs_quad(uint vid : SV_VertexID, Inst I)
{
  VOut o;
  const float2 t = float2(vid & 1, vid >> 1);
  const float2 xy = lerp(I.bounds.xy, I.bounds.zw, t);
  o.pos = float4(xy.x * to_ndc.x - 1.0, 1.0 - xy.y * to_ndc.y, 0.0, 1.0);
  o.rect = I.rect;
  o.c0 = I.c0;
  o.c1 = I.c1;
  o.c2 = I.c2;
  o.c3 = I.c3;
  o.p = I.p;
  o.uv = I.uv;
  return o;
}

// Rounded box with a per-corner radius mask (gpu_raster.c sd_widget_box).
float sd_box(float2 s, float2 c, float2 hb, float rad, uint corners)
{
  const float2 p = s - c;
  const uint bit = p.y < 0.0 ? (p.x < 0.0 ? 1u : 2u) : (p.x < 0.0 ? 8u : 4u);
  const float r = (corners & bit) ? rad : 0.0;
  const float2 q = abs(p) - (hb - r);
  if (q.x <= 0.0 || q.y <= 0.0) {
    return max(q.x, q.y) - r;
  }
  return length(q) - r;
}

// Source-over of a straight color into a premultiplied accumulator.
void over(inout float4 acc, float4 c, float cov)
{
  const float a = saturate(c.a * cov);
  acc.rgb = c.rgb * a + acc.rgb * (1.0 - a);
  acc.a = a + acc.a * (1.0 - a);
}

float4 premul(float4 c, float cov)
{
  const float a = saturate(c.a * cov);
  return float4(c.rgb * a, a);
}

float3 hsv_to_rgb(float h, float s, float v)
{
  h = h - floor(h);
  const float h6 = h * 6.0;
  const float3 rgb = saturate(float3(abs(h6 - 3.0) - 1.0, 2.0 - abs(h6 - 2.0), 2.0 - abs(h6 - 4.0)));
  return ((rgb - 1.0) * s + 1.0) * v;
}

float4 ps_quad(VOut v) : SV_Target
{
  const float2 s = v.pos.xy; // pixel center
  const int kind = (int)v.p.x;
  const float4 R = v.rect;

  if (kind == K_RECT) {
    const float2 px = s - 0.5;
    const float2 lo = max(R.xy, px), hi = min(R.xy + R.zw, px + 1.0);
    const float2 cv = saturate(hi - lo);
    return premul(v.c0, cv.x * cv.y);
  }
  if (kind == K_WIDGET) {
    const float x = R.x, y = R.y, w = R.z, h = R.w;
    const float2 hb = float2(w, h) * 0.5, c = R.xy + hb;
    const float rad = v.p.y, radi = v.p.z, px1 = v.p.w;
    const uint corners = (uint)v.uv.x, flags = (uint)v.uv.y;
    const float ow = v.uv.z;
    float4 acc = 0.0;
    const float d = sd_box(s, c, hb, rad, corners);
    const float co = saturate(0.5 - d);
    if (v.c3.a > 0.0 && s.y > c.y) {
      const float de = sd_box(s - float2(0.0, px1), c, hb, rad, corners);
      const float ec = saturate(0.5 - de) - co;
      if (ec > 0.0) {
        over(acc, v.c3, ec);
      }
    }
    if (co > 0.0) {
      float ci, band;
      float t;
      if (flags & W_LEGACY) {
        ci = co;
        band = ow > 0.0 ? co - saturate(0.5 - (d + ow)) : 0.0;
        t = saturate((s.y - y) / h);
      }
      else {
        const float di = sd_box(s, c, hb - px1, radi, corners);
        ci = min(co, saturate(0.5 - di));
        band = co - ci;
        t = (flags & W_SHADE_H) ? saturate((s.x - (x + px1)) / max(1.0, w - 2.0 * px1)) :
                                  saturate((s.y - (y + px1)) / max(1.0, h - 2.0 * px1));
      }
      if ((flags & W_INNER) && ci > 0.0) {
        if (flags & W_CHECKER) {
          const int cell = ((int)((s.x - x) / 8.0) + (int)((s.y - y) / 8.0)) & 1;
          const float g = cell ? 160.0 / 255.0 : 100.0 / 255.0;
          over(acc, float4(g, g, g, 1.0), ci);
        }
        over(acc, lerp(v.c0, v.c1, t), ci);
      }
      if (v.c2.a > 0.0 && band > 0.0) {
        over(acc, v.c2, band);
      }
    }
    return acc;
  }
  if (kind == K_ELLIPSE) {
    const float lw = v.p.y;
    const float2 dd = s - R.xy;
    float dist;
    if (abs(R.z - R.w) < 1e-3) {
      dist = length(dd) - R.z;
    }
    else {
      const float2 n = dd / R.zw;
      const float k = length(n);
      const float g = length(n / R.zw);
      dist = g > 0.0 ? (k - 1.0) * k / g : -min(R.z, R.w);
    }
    const float cov = lw > 0.0 ? saturate(lw * 0.5 - abs(dist) + 0.5) : saturate(0.5 - dist);
    return premul(v.c0, cov);
  }
  if (kind == K_LINE) {
    const float2 a = R.xy, vv = R.zw - R.xy;
    const float2 wv = s - a;
    const float t = saturate(dot(wv, vv) / dot(vv, vv));
    const float d = length(wv - vv * t) - v.p.y;
    return premul(v.c0, saturate(0.5 - d));
  }
  if (kind == K_MASK) {
    const int2 texel = (int2)v.uv.xy + (int2)(floor(s) - R.xy);
    const float cov = mask_tex.Load(int3(texel, 0));
    return premul(v.c0, cov);
  }
  if (kind == K_IMAGE) {
    const float2 isz = v.uv.zw;
    const int2 sp = clamp((int2)((s - R.xy) * isz / R.zw), int2(0, 0), (int2)isz - 1);
    const float4 c = rgba_tex.Load(int3((int2)v.uv.xy + sp, 0));
    return premul(c, v.p.y);
  }
  if (kind == K_SHADOW) {
    const float rad = v.p.y, size = v.p.z, alpha = v.p.w;
    const float2 hb = R.zw * 0.5;
    const float2 c = R.xy + hb;
    const float d = sd_box(s, c + float2(0.0, size * 0.25), hb, rad, 15u);
    const float dbox = sd_box(s, c, hb, rad, 15u);
    if (dbox <= -0.5 || d >= size) {
      return 0.0;
    }
    float t = 1.0 - saturate(d / size);
    t = t * t * (3.0 - 2.0 * t);
    return float4(0.0, 0.0, 0.0, saturate(t * alpha * saturate(0.5 + dbox)));
  }
  if (kind == K_GRADIENT) {
    const float2 uv = saturate((s - R.xy) / R.zw);
    const float4 left = lerp(v.c0, v.c3, uv.y), right = lerp(v.c1, v.c2, uv.y);
    return premul(lerp(left, right, uv.x), 1.0);
  }
  if (kind == K_HSV) {
    const float2 dd = s - R.xy;
    const float dist = length(dd);
    const float cov = saturate(R.z - dist + 0.5);
    const float hue = atan2(dd.x, -dd.y) * 0.15915494309189535 + 0.5;
    return premul(float4(hsv_to_rgb(hue, min(dist / R.z, 1.0), R.w), 1.0), cov);
  }
  if (kind == K_SPHERE) {
    const float2 n2 = float2(s.x - R.x, -(s.y - R.y)) / R.z;
    const float d2 = dot(n2, n2);
    const float cov = saturate((1.0 - sqrt(d2)) * R.z + 0.5);
    const float3 n = float3(n2, sqrt(max(0.0, 1.0 - d2)));
    const float diff = max(0.0, dot(n, v.c1.xyz));
    float spec = max(0.0, dot(n, v.c2.xyz));
    spec = pow(spec, 16.0); // the software raster squares 2x then takes ^4: ^16
    return premul(float4(v.c0.rgb * (0.12 + 0.88 * diff) + 0.6 * spec, v.c0.a), cov);
  }
  return 0.0;
}

// Per-vertex colored triangles (the BuiGpuSink seam), clipped per vertex.
struct TriIn {
  float2 pos : POSITION;
  float4 color : COLOR0;
  float4 clip : CLIP;
};
struct TriOut {
  float4 pos : SV_Position;
  float4 color : COLOR0;
  nointerpolation float4 clip : CLIP;
};

TriOut vs_tri(TriIn i)
{
  TriOut o;
  o.pos = float4(i.pos.x * to_ndc.x - 1.0, 1.0 - i.pos.y * to_ndc.y, 0.0, 1.0);
  o.color = i.color;
  o.clip = i.clip;
  return o;
}

float4 ps_tri(TriOut i) : SV_Target
{
  if (i.pos.x < i.clip.x || i.pos.y < i.clip.y || i.pos.x >= i.clip.z || i.pos.y >= i.clip.w) {
    discard;
  }
  return premul(i.color, 1.0);
}
