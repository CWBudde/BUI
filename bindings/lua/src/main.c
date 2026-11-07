/* bui-lua: standalone Lua 5.4 host for the BUI toolkit.
 * Entry point: CLI parsing, script execution, REPL.
 * SPDX-License-Identifier: GPL-2.0-or-later */
#include "bui_internal.h"
#include "linenoise.h"
#include <string.h>
#include <stdlib.h>

/* Forward: REPL */
static void run_repl(lua_State *L);

/* Forward: eval inline code */
static int run_eval(lua_State *L, const char *code);

/* ------------------------------------------------------------------ */
/* Help & version                                                     */
/* ------------------------------------------------------------------ */

static void print_help(void) {
  printf("Usage: bui-lua [options] [script [args...]]\n");
  printf("\n");
  printf("Options:\n");
  printf("  -e <expr>       Execute inline Lua code\n");
  printf("  --headless      Force software raster mode (no display needed)\n");
  printf("  --theme <name>  Set initial theme (dark | light)\n");
  printf("  --scale <n>     Set UI scale factor\n");
  printf("  --version       Print version and exit\n");
  printf("  --help          Print this help and exit\n");
  printf("\n");
  printf("Script mode:\n");
  printf("  bui-lua app.lua           Run app.lua\n");
  printf("  bui-lua -e \"code\"         Run inline code\n");
  printf("\n");
  printf("REPL mode:\n");
  printf("  bui-lua                   Interactive REPL prompt\n");
}

static void print_version(void) {
  int major, minor, patch;
  bui_get_version(&major, &minor, &patch);
  printf("bui-lua %d.%d.%d\n", major, minor, patch);
}

/* ------------------------------------------------------------------ */
/* Main                                                                */
/* ------------------------------------------------------------------ */

int main(int argc, char **argv) {
  const char *script = NULL;
  const char *eval_code = NULL;
  int headless = 0;
  const char *theme = NULL;
  float scale = 0;

  /* Parse options */
  int i = 1;
  while (i < argc && argv[i][0] == '-') {
    if (strcmp(argv[i], "--help") == 0) {
      print_help();
      return 0;
    }
    else if (strcmp(argv[i], "--version") == 0) {
      print_version();
      return 0;
    }
    else if (strcmp(argv[i], "--headless") == 0) {
      headless = 1;
      i++;
    }
    else if (strcmp(argv[i], "--theme") == 0 && i + 1 < argc) {
      theme = argv[++i];
      i++;
    }
    else if (strcmp(argv[i], "--scale") == 0 && i + 1 < argc) {
      scale = (float)atof(argv[++i]);
      i++;
    }
    else if (strcmp(argv[i], "-e") == 0 && i + 1 < argc) {
      eval_code = argv[++i];
      i++;
    }
    else {
      fprintf(stderr, "bui-lua: unknown option '%s'\n", argv[i]);
      fprintf(stderr, "Try 'bui-lua --help' for usage.\n");
      return 1;
    }
  }

  /* Remaining args: first is script, rest go into arg table */
  if (i < argc && !eval_code) {
    script = argv[i];
    i++;
    /* Remaining args after script are passed to the script */
  }

  /* Headless mode: check if window system is needed */
  if (headless) {
    /* Software raster works without a display */
  }

  /* Create Lua state */
  lua_State *L = luaL_newstate();
  if (!L) {
    fprintf(stderr, "bui-lua: failed to create Lua state\n");
    return 1;
  }
  luaL_openlibs(L);

  /* Register the C module. For the standalone executable this directly
   * registers luaopen_bui under the name "bui". When loaded as a shared
   * library, require("bui") finds bui.lua (the wrapper) which calls
   * require("_bui") to get the C bindings underneath. */
  luaL_requiref(L, "bui", luaopen_bui, 1);
  lua_pop(L, 1);

  /* Set theme before any UI setup */
  if (theme) {
    if (!BK_set_theme(theme)) {
      fprintf(stderr, "bui-lua: unknown theme '%s', using default\n", theme);
    }
  }

  /* Set scale */
  if (scale > 0) {
    BK_set_scale(scale);
  }

  /* Execute */
  int status = 0;

  if (eval_code) {
    /* Inline eval */
    status = run_eval(L, eval_code);
  }
  else if (script) {
    /* Set arg table */
    lua_createtable(L, argc - i, 0);
    for (int j = i; j < argc; j++) {
      lua_pushstring(L, argv[j]);
      lua_rawseti(L, -2, j - i + 1);
    }
    lua_setglobal(L, "arg");

    /* Load and run script */
    if (luaL_dofile(L, script) != LUA_OK) {
      fprintf(stderr, "bui-lua: %s\n", lua_tostring(L, -1));
      lua_pop(L, 1);
      status = 1;
    }
  }
  else {
    /* REPL */
    print_version();
    printf("Type 'exit' to quit\n");
    run_repl(L);
  }

  lua_close(L);
  return status;
}

/* ------------------------------------------------------------------ */
/* Inline evaluation                                                   */
/* ------------------------------------------------------------------ */

static int run_eval(lua_State *L, const char *code) {
  if (luaL_dostring(L, code) != LUA_OK) {
    fprintf(stderr, "bui-lua: %s\n", lua_tostring(L, -1));
    lua_pop(L, 1);
    return 1;
  }
  return 0;
}

/* ------------------------------------------------------------------ */
/* REPL                                                                 */
/* ------------------------------------------------------------------ */

/* Try to execute a line of input. Returns 1 if the input was complete
 * and executed, 0 if more lines are needed (incomplete input). */
static int try_exec(lua_State *L, const char *buf, size_t len) {
  int load_status = luaL_loadbuffer(L, buf, len, "=stdin");

  if (load_status == LUA_ERRSYNTAX) {
    const char *msg = lua_tostring(L, -1);
    if (msg) {
      size_t msglen = strlen(msg);
      /* Check for incomplete input: "near '<eof>'" at the end */
      if (msglen >= 11 && strcmp(msg + msglen - 11, "near '<eof>'") == 0) {
        lua_pop(L, 1);
        return 0; /* need more input */
      }
    }
    /* Real syntax error */
    fprintf(stderr, "error: %s\n", lua_tostring(L, -1));
    lua_pop(L, 1);
    return 1;
  }

  if (load_status != LUA_OK) {
    fprintf(stderr, "error: %s\n", lua_tostring(L, -1));
    lua_pop(L, 1);
    return 1;
  }

  /* Execute */
  if (lua_pcall(L, 0, LUA_MULTRET, 0) != LUA_OK) {
    fprintf(stderr, "error: %s\n", lua_tostring(L, -1));
    lua_pop(L, 1);
    return 1;
  }

  /* Print return values */
  int n = lua_gettop(L);
  if (n > 0) {
    for (int i = 1; i <= n; i++) {
      if (i > 1) printf("\t");
      switch (lua_type(L, i)) {
        case LUA_TNIL:
          printf("nil"); break;
        case LUA_TBOOLEAN:
          printf("%s", lua_toboolean(L, i) ? "true" : "false"); break;
        case LUA_TNUMBER:
          printf("%.14g", lua_tonumber(L, i)); break;
        case LUA_TSTRING:
          printf("%s", lua_tostring(L, i)); break;
        case LUA_TTABLE:
          printf("table: %p", lua_topointer(L, i)); break;
        case LUA_TFUNCTION:
          printf("function: %p", lua_topointer(L, i)); break;
        case LUA_TUSERDATA:
        {
          /* Check if it's one of our types */
          if (luaL_testudata(L, i, LUABUI_NODE_MT))
            printf("bui.Node");
          else if (luaL_testudata(L, i, LUABUI_CONTEXT_MT))
            printf("bui.Context");
          else if (luaL_testudata(L, i, LUABUI_WINDOW_MT))
            printf("bui.Window");
          else
            printf("userdata: %p", lua_topointer(L, i));
          break;
        }
        default:
          printf("%s: %p", lua_typename(L, lua_type(L, i)), lua_topointer(L, i));
          break;
      }
    }
    printf("\n");
  }
  lua_settop(L, 0);
  return 1;
}

static void run_repl(lua_State *L) {
  linenoiseHistorySetMaxLen(100);
  linenoiseHistoryLoad("~/.bui-lua-history");

  /* Multi-line input buffer */
  char *multiline = NULL;
  size_t multiline_len = 0;

  char *line;
  while ((line = linenoise("bui> ")) != NULL) {
    if (strcmp(line, "exit") == 0 || strcmp(line, "quit") == 0) {
      free(line);
      break;
    }

    linenoiseHistoryAdd(line);
    linenoiseHistorySave("~/.bui-lua-history");

    /* Add to multi-line buffer */
    size_t line_len = strlen(line);
    char *newbuf = (char *)realloc(multiline, multiline_len + line_len + 2);
    if (!newbuf) {
      free(line);
      break;
    }
    multiline = newbuf;
    if (multiline_len > 0) {
      multiline[multiline_len] = '\n';
      multiline[multiline_len + 1] = '\0';
      multiline_len++;
    } else {
      multiline[0] = '\0';
    }
    memcpy(multiline + multiline_len, line, line_len + 1);
    multiline_len += line_len;

    /* Try to execute */
    if (try_exec(L, multiline, multiline_len)) {
      /* Success — clear buffer */
      free(multiline);
      multiline = NULL;
      multiline_len = 0;
    }
    /* If incomplete, next iteration reads more lines */

    free(line);
  }

  free(multiline);
  printf("\n");
}
