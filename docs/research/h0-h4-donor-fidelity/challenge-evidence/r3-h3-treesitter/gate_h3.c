/* Gate-A R3 upstream qualitative cross-check harness.
 * tree-sitter v0.27.0 core @ 6070dbf + tree-sitter-markdown v0.5.3 block grammar
 * @ f969cd3. QUALITATIVE ONLY: prints reuse/changed-range outcomes via the
 * parser's own reuse LOG and ts_tree_get_changed_ranges. No timing of any
 * kind is recorded or performed.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "tree_sitter/api.h"

TSLanguage *tree_sitter_markdown(void);

static TSPoint point_of(const char *s, uint32_t byte) {
  TSPoint p = {0, 0};
  for (uint32_t i = 0; i < byte; i++) {
    if (s[i] == '\n') { p.row++; p.column = 0; } else p.column++;
  }
  return p;
}

static unsigned g_reuse_events;
static unsigned g_cant_reuse_events;
static char g_last_reused[8][64];
static unsigned g_last_reused_n;
static char g_cant_reasons[6][80];
static unsigned g_cant_n;

static void log_sink(void *payload, TSLogType type, const char *msg) {
  (void)payload; (void)type;
  if (strncmp(msg, "reuse_node ", 11) == 0) {
    g_reuse_events++;
    if (g_last_reused_n < 8)
      snprintf(g_last_reused[g_last_reused_n++], 64, "%s", msg + 11);
  } else if (strncmp(msg, "cant_reuse_node", 15) == 0) {
    g_cant_reuse_events++;
    if (g_cant_n < 6) {
      const char *r = strstr(msg, "cant_reuse_node");
      snprintf(g_cant_reasons[g_cant_n++], 80, "%s", r);
    }
  }
}

static void scenario(const char *name, const char *old_src,
                     uint32_t es, uint32_t ee, const char *inserted) {
  TSParser *parser = ts_parser_new();
  ts_parser_set_language(parser, tree_sitter_markdown());

  TSTree *old_tree = ts_parser_parse_string(parser, NULL, old_src, strlen(old_src));
  if (!old_tree) { printf("== %s: OLD PARSE FAILED ==\n", name); ts_parser_delete(parser); return; }

  uint32_t ins_len = strlen(inserted);
  uint32_t new_len = strlen(old_src) - (ee - es) + ins_len;
  char *post = malloc(new_len + 1);
  memcpy(post, old_src, es);
  memcpy(post + es, inserted, ins_len);
  memcpy(post + es + ins_len, old_src + ee, strlen(old_src) - ee);
  post[new_len] = 0;

  TSInputEdit edit = {
    .start_byte = es,
    .old_end_byte = ee,
    .new_end_byte = es + ins_len,
    .start_point = point_of(old_src, es),
    .old_end_point = point_of(old_src, ee),
    .new_end_point = point_of(post, es + ins_len),
  };
  ts_tree_edit(old_tree, &edit);

  printf("== %s ==\n", name);
  printf("  doc: old %zu bytes, post %u bytes\n", strlen(old_src), new_len);

  /* Observed incremental parse WITH the reuse log. */
  g_reuse_events = 0; g_cant_reuse_events = 0; g_last_reused_n = 0; g_cant_n = 0;
  {
    TSParser *p2 = ts_parser_new();
    ts_parser_set_language(p2, tree_sitter_markdown());
    TSLogger logger = {.payload = NULL, .log = log_sink};
    ts_parser_set_logger(p2, logger);
    TSTree *t2 = ts_parser_parse_string(p2, old_tree, post, new_len);
    if (!t2) printf("  INCREMENTAL PARSE FAILED\n");
    else ts_tree_delete(t2);
    ts_parser_delete(p2);
  }
  printf("  mechanism log: reuse_node events = %u, cant_reuse events = %u\n",
         g_reuse_events, g_cant_reuse_events);
  for (unsigned i = 0; i < g_last_reused_n && i < 8; i++)
    printf("    reused: %s\n", g_last_reused[i]);
  for (unsigned i = 0; i < g_cant_n; i++)
    printf("    refused: %s\n", g_cant_reasons[i]);

  /* Plain incremental parse for changed-ranges reporting. */
  TSTree *new_tree = ts_parser_parse_string(parser, old_tree, post, new_len);
  if (!new_tree) { printf("  INCREMENTAL PARSE FAILED (2)\n"); goto done; }

  uint32_t n_cr = 0;
  TSRange *cr = ts_tree_get_changed_ranges(old_tree, new_tree, &n_cr);
  printf("  changed_ranges(old_edited -> new): %u", n_cr);
  uint32_t cov = 0;
  for (uint32_t i = 0; i < n_cr; i++) {
    printf(" [%u,%u)", cr[i].start_byte, cr[i].end_byte);
    cov += cr[i].end_byte - cr[i].start_byte;
  }
  printf("\n  changed coverage: %u / %u bytes => %s\n", cov, new_len,
         cov == 0 ? "FULL REUSE" : (cov < new_len / 4 ? "LOCAL REUSE" : (cov < new_len ? "PARTIAL" : "NO REUSE")));
  if (cr) free(cr);

  /* upstream self-equivalence: incremental tree vs clean reparse */
  TSTree *clean_tree = ts_parser_parse_string(parser, NULL, post, new_len);
  if (clean_tree) {
    uint32_t n_d = 0;
    TSRange *d = ts_tree_get_changed_ranges(new_tree, clean_tree, &n_d);
    printf("  incremental vs clean reparse: %u differing ranges\n", n_d);
    if (d) free(d);
    ts_tree_delete(clean_tree);
  }
  ts_tree_delete(new_tree);

done:
  free(post);
  ts_tree_delete(old_tree);
  ts_parser_delete(parser);
}

int main(void) {
  /* F1: local edit inside one paragraph of a 10-paragraph document */
  {
    const char *para = "Lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod.\n\n";
    char doc[4096] = {0};
    for (int i = 0; i < 10; i++) strcat(doc, para);
    uint32_t pos = (uint32_t)(strlen(para) * 5 + 10);
    scenario("F1 local edit block5 of 10-para doc", doc, pos, pos + 3, "XYZ");
  }

  /* F2: unclosed fence inserted at the start; all later bytes unchanged but
   * scanner state flips everywhere. */
  {
    const char *old = "alpha one\n\nfiller line one to push length beyond any boundary\n\n"
                      "filler line two keeps tail far away\n\nbeta two\n";
    scenario("F2 unclosed fence insert flips state", old, 0, 0, "```\n");
  }

  /* F3: edit overlapping the second paragraph */
  {
    const char *old = "para one\n\npara two\n\npara three\n\npara four\n";
    uint32_t p2 = 10;
    scenario("F3 edit overlaps block two", old, p2, p2 + 8, "EDITED TWO");
  }

  /* F4: one quote with three real paragraphs; edit inside the third */
  {
    const char *old = "> alpha beta gamma delta epsilon zeta eta theta\n"
                      ">\n"
                      "> second paragraph with more words to fill size out\n"
                      ">\n"
                      "> third paragraph with even more filler words to pass\n"
                      "\n"
                      "after the quote para\n";
    const char *hit = strstr(old, "third paragraph");
    uint32_t p = (uint32_t)(hit - old) + 5;
    scenario("F4 edit inside 3rd para of one quote", old, p, p + 6, "TWEAK");
  }

  /* F4b: list with six items; edit inside item 3 */
  {
    char doc[2048] = {0};
    for (int i = 0; i < 6; i++) {
      char line[128];
      snprintf(line, sizeof line, "- item number %d with some filler text here\n", i);
      strcat(doc, line);
    }
    strcat(doc, "\nafter para\n");
    const char *hit = strstr(doc, "item number 3");
    uint32_t p = (uint32_t)(hit - doc) + 6;
    scenario("F4b edit inside item 3 of 6-item list", doc, p, p + 6, "CHANGE");
  }

  /* F5: 200 blocks, edit near the end */
  {
    char *doc = malloc(200 * 60 + 16);
    doc[0] = 0;
    for (int i = 0; i < 200; i++) {
      char line[64];
      snprintf(line, sizeof line, "block %03d text fill fill fill fill fill fill.\n\n", i);
      strcat(doc, line);
    }
    uint32_t late = (uint32_t)strlen(doc) - 60;
    scenario("F5 200 blocks, late edit", doc, late, late + 3, "ZED");
    free(doc);
  }
  return 0;
}
