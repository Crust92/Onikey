/*
 * onikey.h — mặt cắt C của lõi xử lý tiếng Việt Onikey.
 *
 * Dùng cho các adapter: addon C++ của Fcitx5, engine IBus, XIM...
 *
 * Quy ước:
 *   - Chuỗi trả về do Rust cấp phát; trả lại bằng onikey_string_free(),
 *     KHÔNG dùng free() của libc.
 *   - Mọi hàm chịu được con trỏ NULL.
 *   - key là MÃ UNICODE của ký tự, không phải keycode bàn phím.
 *
 * Giấy phép: GPL-3.0-or-later
 */
#ifndef ONIKEY_H
#define ONIKEY_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct OnikeyEngine OnikeyEngine;

/* Cờ của lõi */
#define ONIKEY_FLAG_FREE_TONE_MARKING 1u
#define ONIKEY_FLAG_STD_TONE_STYLE 2u
#define ONIKEY_FLAG_AUTO_CORRECT 4u
#define ONIKEY_FLAG_STD                                                        \
  (ONIKEY_FLAG_FREE_TONE_MARKING | ONIKEY_FLAG_STD_TONE_STYLE |                \
   ONIKEY_FLAG_AUTO_CORRECT)

/* Chế độ trải chuỗi */
#define ONIKEY_MODE_VIETNAMESE 1u
#define ONIKEY_MODE_ENGLISH 2u
#define ONIKEY_MODE_TONE_LESS 4u
#define ONIKEY_MODE_MARK_LESS 8u
#define ONIKEY_MODE_LOWER_CASE 16u
#define ONIKEY_MODE_FULL_TEXT 32u
#define ONIKEY_MODE_PUNCTUATION 64u
#define ONIKEY_MODE_IN_REVERSE_ORDER 128u

OnikeyEngine *onikey_engine_new(const char *input_method, unsigned int flags);
void onikey_engine_free(OnikeyEngine *engine);
void onikey_engine_reset(OnikeyEngine *engine);

void onikey_engine_process_key(OnikeyEngine *engine, uint32_t key,
                               unsigned int mode);
char *onikey_engine_get_string(const OnikeyEngine *engine, unsigned int mode);
bool onikey_engine_is_valid(const OnikeyEngine *engine, bool full);
bool onikey_engine_can_process_key(const OnikeyEngine *engine, uint32_t key);
void onikey_engine_remove_last_char(OnikeyEngine *engine, bool refresh_tone);
void onikey_engine_restore_last_word(OnikeyEngine *engine, bool to_vietnamese);

char *onikey_encode(const char *charset, const char *input);

/* Chuỗi hiển thị: tiếng Việt bỏ dấu, hoặc phím gốc nếu từ không phải tiếng
 * Việt (auto_restore). Logic dùng chung mọi adapter — đừng tự chế lại. */
char *onikey_engine_display_string(const OnikeyEngine *engine,
                                   bool auto_restore, bool dd_free_style);

/* Cấu hình người dùng (~/.config/onikey/onikey.config.json) */
typedef struct OnikeyUserConfig {
  char input_method[64];
  char output_charset[64];
  unsigned int core_flags;
  unsigned int ib_flags;
  unsigned int default_input_mode; /* 1 = Pre-edit */
} OnikeyUserConfig;

bool onikey_load_user_config(OnikeyUserConfig *out);

#define ONIKEY_IBFLAG_AUTO_NON_VN_RESTORE (1u << 5)
#define ONIKEY_IBFLAG_DD_FREE_STYLE (1u << 6)

void onikey_string_free(char *s);

#ifdef __cplusplus
}
#endif

#endif /* ONIKEY_H */
