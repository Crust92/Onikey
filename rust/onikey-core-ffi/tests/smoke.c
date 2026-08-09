/* Kiểm tra vỏ C thật sự dùng được từ C: biên dịch, liên kết tĩnh, gõ, đối chiếu. */
#include <stdio.h>
#include <string.h>
#include "onikey.h"

static int fails = 0;

static void check(const char *ten, const char *got, const char *want) {
  if (strcmp(got, want) != 0) {
    printf("  LỆCH %s: nhận \"%s\", mong đợi \"%s\"\n", ten, got, want);
    fails++;
  } else {
    printf("  ok  %s -> %s\n", ten, got);
  }
}

static void go(OnikeyEngine *e, const char *keys) {
  for (const char *p = keys; *p; p++)
    onikey_engine_process_key(e, (uint32_t)*p, ONIKEY_MODE_VIETNAMESE);
}

int main(void) {
  OnikeyEngine *e = onikey_engine_new("Telex", ONIKEY_FLAG_STD);
  go(e, "tieengs");
  char *s = onikey_engine_get_string(e, ONIKEY_MODE_VIETNAMESE);
  check("telex tieengs", s, "tiếng");
  onikey_string_free(s);

  onikey_engine_reset(e);
  go(e, "Vieejt");
  s = onikey_engine_get_string(e, ONIKEY_MODE_VIETNAMESE);
  check("giữ chữ hoa", s, "Việt");
  onikey_string_free(s);

  s = onikey_engine_get_string(e, ONIKEY_MODE_ENGLISH | ONIKEY_MODE_FULL_TEXT);
  check("khôi phục phím gốc", s, "Vieejt");
  onikey_string_free(s);

  onikey_engine_remove_last_char(e, true);
  s = onikey_engine_get_string(e, ONIKEY_MODE_VIETNAMESE);
  check("xoá lùi", s, "Việ");
  onikey_string_free(s);
  onikey_engine_free(e);

  OnikeyEngine *v = onikey_engine_new("VNI", ONIKEY_FLAG_STD);
  go(v, "tie61ng");
  s = onikey_engine_get_string(v, ONIKEY_MODE_VIETNAMESE);
  check("vni tie61ng", s, "tiếng");
  onikey_string_free(s);
  onikey_engine_free(v);

  s = onikey_encode("TCVN3 (ABC)", "tiếng Việt");
  printf("  bảng mã TCVN3: %s\n", s);
  onikey_string_free(s);

  /* con trỏ rỗng không được làm sập */
  onikey_engine_free(NULL);
  onikey_engine_process_key(NULL, 'a', 1);
  onikey_string_free(NULL);

  printf(fails ? "CÓ LỖI\n" : "TẤT CẢ ĐỀU ĐÚNG\n");
  return fails != 0;
}
