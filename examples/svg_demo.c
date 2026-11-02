/* BUI example: draw a showcase scene through the GPU sink into an SVG file.
 *
 * Demonstrates the sink dispatch: the same primitives could be rendered by
 * the Qt backend (backends/qt) or a Python backend (bindings/python).
 *
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bui_c_api.h"
#include "GPU_immediate.hh"

typedef struct SvgRecorder {
  FILE *file;
  int width;
  int height;
} SvgRecorder;

static void hex_color(const float rgba[4], char out[8])
{
  for (int i = 0; i < 3; i++) {
    float c = rgba[i];
    c = c < 0.0f ? 0.0f : (c > 1.0f ? 1.0f : c);
    snprintf(out + i * 2, 3, "%02x", (unsigned int)(c * 255.0f + 0.5f));
  }
  out[6] = '\0';
}

static void rec_triangles(void *user, const float (*coords)[2], const float (*colors)[4], int count)
{
  SvgRecorder *rec = user;
  for (int i = 0; i + 2 < count; i += 3) {
    char hex[8];
    hex_color(colors[i], hex);
    fprintf(rec->file,
            "<polygon points=\"%.1f,%.1f %.1f,%.1f %.1f,%.1f\" fill=\"#%s\"/>\n",
            coords[i][0], coords[i][1], coords[i + 1][0], coords[i + 1][1], coords[i + 2][0],
            coords[i + 2][1], hex);
  }
}

static void rec_lines(void *user, const float (*coords)[2], const float (*colors)[4], int count)
{
  SvgRecorder *rec = user;
  for (int i = 0; i + 1 < count; i += 2) {
    char hex[8];
    hex_color(colors[i], hex);
    fprintf(rec->file,
            "<line x1=\"%.1f\" y1=\"%.1f\" x2=\"%.1f\" y2=\"%.1f\" stroke=\"#%s\" "
            "stroke-width=\"1\"/>\n",
            coords[i][0], coords[i][1], coords[i + 1][0], coords[i + 1][1], hex);
  }
}

static void rec_points(void *user, const float (*coords)[2], const float (*colors)[4], int count, float size)
{
  SvgRecorder *rec = user;
  for (int i = 0; i < count; i++) {
    char hex[8];
    hex_color(colors[i], hex);
    fprintf(rec->file, "<circle cx=\"%.1f\" cy=\"%.1f\" r=\"%.1f\" fill=\"#%s\"/>\n",
            coords[i][0], coords[i][1], size > 0 ? size / 2.0f : 0.5f, hex);
  }
}

static void rec_text(void *user, float x, float y, const char *text, float size, const float rgba[4])
{
  SvgRecorder *rec = user;
  char hex[8];
  hex_color(rgba, hex);
  fprintf(rec->file, "<text x=\"%.1f\" y=\"%.1f\" font-size=\"%.0f\" fill=\"#%s\">%s</text>\n",
          x, y, size, hex, text);
}

static void rec_begin(void *user, int w, int h)
{
  SvgRecorder *rec = user;
  rec->width = w;
  rec->height = h;
  fprintf(rec->file,
          "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"%d\" height=\"%d\">\n",
          w, h);
}

static void rec_end(void *user)
{
  SvgRecorder *rec = user;
  (void)rec;
  fprintf(rec->file, "</svg>\n");
}

int main(int argc, char **argv)
{
  const char *out_path = argc > 1 ? argv[1] : "bui_svg_demo.svg";

  SvgRecorder rec;
  rec.file = fopen(out_path, "w");
  rec.width = 640;
  rec.height = 480;
  if (!rec.file) {
    perror("fopen");
    return 1;
  }

  BuiGpuSink sink = {0};
  sink.user = &rec;
  sink.begin_frame = rec_begin;
  sink.triangles = rec_triangles;
  sink.lines = rec_lines;
  sink.points = rec_points;
  sink.text = rec_text;
  sink.end_frame = rec_end;
  bui_set_render_sink(&sink);

  /* Draw through the BUI immediate-mode seam; the sink receives everything. */
  GPUVertFormat *fmt = immVertexFormat();
  unsigned int pos_id = GPU_vertformat_attr_add(fmt, "pos", SFLOAT_32_32);
  unsigned int col_id = GPU_vertformat_attr_add(fmt, "color", SFLOAT_32_32_32_32);

  immBindBuiltinProgram(GPU_SHADER_2D_SMOOTH_COLOR);
  immBegin(GPU_PRIM_TRIS, 3);
  immAttr4f(col_id, 0.25f, 0.47f, 0.85f, 1.0f);
  immVertex2f(pos_id, 320.0f, 60.0f);
  immAttr4f(col_id, 0.85f, 0.35f, 0.35f, 1.0f);
  immVertex2f(pos_id, 120.0f, 420.0f);
  immAttr4f(col_id, 0.35f, 0.8f, 0.45f, 1.0f);
  immVertex2f(pos_id, 520.0f, 420.0f);
  immEnd();

  immBegin(GPU_PRIM_LINE_STRIP, 4);
  for (int i = 0; i < 4; i++) {
    immAttr4f(col_id, 0.95f, 0.85f, 0.4f, 1.0f);
    immVertex2f(pos_id, 100.0f + i * 60.0f, 450.0f - i * 8.0f);
  }
  immEnd();
  immUnbindProgram();

  bui_set_render_sink(NULL);
  fclose(rec.file);
  printf("SVG written to %s\n", out_path);
  return 0;
}
