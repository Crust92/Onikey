/*
 * Onikey - sinh bảng mã cho lõi Rust từ chính bảng của bản Go
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 */

// gen-charsets SINH file Rust chứa bảng chuyển bảng mã (TCVN3, VNI Windows,
// VIQR, VPS, VISCII, BK HCM…) từ chính bảng của bamboo-core.
//
//	go run ./tools/gen-charsets > rust/onikey-core/src/charsets.rs
//
// Chép tay hơn hai nghìn dòng ánh xạ ký tự là mời lỗi chính tả vào nhà, mà lỗi
// kiểu đó chỉ lộ ra khi người dùng xuất ra bảng mã cũ rồi thấy sai một chữ.
// Sinh máy thì sai hay đúng cũng sai/đúng đồng loạt và kiểm được bằng test.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"
	"strings"

	bamboo "github.com/BambooEngine/bamboo-core"
)

func rustStr(s string) string {
	var b strings.Builder
	b.WriteByte('"')
	for _, r := range s {
		switch r {
		case '"':
			b.WriteString("\\\"")
		case '\\':
			b.WriteString("\\\\")
		default:
			if r < 0x20 || (r >= 0x7f && r <= 0xa0) {
				fmt.Fprintf(&b, "\\u{%x}", r)
			} else {
				b.WriteRune(r)
			}
		}
	}
	b.WriteByte('"')
	return b.String()
}

func main() {
	var names = bamboo.GetCharsetNames()
	sort.Strings(names)

	// Chế độ "fixture": in ra ca kiểm (bảng mã, chuỗi vào, chuỗi ra) để bản
	// Rust đối chiếu — kiểm cả hàm encode chứ không chỉ bảng tra.
	if len(os.Args) > 1 && os.Args[1] == "fixture" {
		var samples = []string{
			"tiếng Việt", "Cộng hoà Xã hội Chủ nghĩa Việt Nam",
			"đường phượng bay mù không lối vào", "ĐƯỜNG", "Ưu tú", "quyển sách",
			"abc123 !@#", "nghiêng ngả", "Tiếng Việt xin chào",
		}
		for r := rune(0x20); r < 0x1f00; r++ {
			samples = append(samples, string(r))
		}
		var enc = json.NewEncoder(os.Stdout)
		for _, name := range names {
			for _, s := range samples {
				enc.Encode(map[string]string{
					"charset": name, "in": s, "out": bamboo.Encode(name, s),
				})
			}
		}
		return
	}

	var out = os.Stdout
	fmt.Fprintln(out, "// TỆP NÀY DO MÁY SINH — ĐỪNG SỬA TAY.")
	fmt.Fprintln(out, "// Sinh lại bằng: go run ./tools/gen-charsets > rust/onikey-core/src/charsets.rs")
	fmt.Fprintln(out, "//")
	fmt.Fprintln(out, "// Bảng chuyển từ Unicode sang các bảng mã tiếng Việt cũ, lấy nguyên từ")
	fmt.Fprintln(out, "// bamboo-core để hai bản cho ra byte y hệt nhau.")
	fmt.Fprintln(out)
	fmt.Fprintln(out, `pub const UNICODE: &str = "Unicode";`)
	fmt.Fprintln(out)
	fmt.Fprintln(out, "/// (tên bảng mã, [(ký tự Unicode, chuỗi thay thế)])")
	fmt.Fprintln(out, "pub const CHARSETS: &[(&str, &[(char, &str)])] = &[")

	for _, name := range names {
		if name == bamboo.UNICODE {
			continue
		}
		// Lấy bảng bằng cách hỏi Encode từng ký tự: bảng gốc không xuất ra ngoài.
		var pairs [][2]string
		for r := rune(0); r < 0x1f00; r++ {
			var in = string(r)
			var enc = bamboo.Encode(name, in)
			if enc != in {
				pairs = append(pairs, [2]string{in, enc})
			}
		}
		fmt.Fprintf(out, "    (%s, &[\n", rustStr(name))
		for _, p := range pairs {
			fmt.Fprintf(out, "        ('%s', %s),\n",
				strings.ReplaceAll(strings.ReplaceAll(p[0], "\\", "\\\\"), "'", "\\'"),
				rustStr(p[1]))
		}
		fmt.Fprintln(out, "    ]),")
	}
	fmt.Fprintln(out, "];")

	fmt.Fprintln(out, `
/// Chuyển chuỗi Unicode sang bảng mã khác. Tên lạ hoặc "Unicode" thì trả nguyên.
pub fn encode(charset_name: &str, input: &str) -> String {
    if charset_name == UNICODE {
        return input.to_string();
    }
    let table = match CHARSETS.iter().find(|(n, _)| *n == charset_name) {
        Some((_, t)) => *t,
        None => return input.to_string(),
    };
    let mut out = String::with_capacity(input.len());
    for chr in input.chars() {
        match table.iter().find(|(c, _)| *c == chr) {
            Some((_, s)) => out.push_str(s),
            None => out.push(chr),
        }
    }
    out
}

pub fn charset_names() -> Vec<&'static str> {
    let mut names = vec![UNICODE];
    names.extend(CHARSETS.iter().map(|(n, _)| *n));
    names
}`)
}
