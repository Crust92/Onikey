/*
 * Onikey - sinh bộ ca kiểm đối chiếu cho lõi tiếng Việt
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

// gen-corpus sinh bảng ca kiểm "(kiểu gõ, chuỗi phím) -> chuỗi ra" từ bản cài
// đặt Go hiện tại, làm LƯỚI AN TOÀN cho việc chuyển lõi sang Rust: bản Rust phải
// cho ra đúng từng ký tự như bảng này, ở TỪNG PHÍM chứ không chỉ kết quả cuối.
//
//	go run ./tools/gen-corpus > tests/corpus/core.jsonl
//
// Cách sinh là LIỆT KÊ XUÔI (gõ phím rồi ghi lại kết quả), không cần từ điển:
// mục tiêu là hai bản cài đặt giống hệt nhau, không phải "tiếng Việt đúng".
package main

import (
	"bufio"
	"encoding/json"
	"fmt"
	"os"
	"sort"

	bamboo "github.com/BambooEngine/bamboo-core"
)

type Case struct {
	IM    string `json:"im"`
	Flags uint   `json:"flags"`
	Keys  string `json:"keys"`
	// Steps: chuỗi tiếng Việt SAU TỪNG PHÍM — chỗ này mới bắt được sai lệch
	// giữa chừng, thứ mà chỉ so kết quả cuối sẽ bỏ lọt.
	Steps []string `json:"steps"`
	Vi    string   `json:"vi"`
	Raw   string   `json:"raw"`
	Valid bool     `json:"valid"`
	// Chỉ có ở bộ ca kiểm tay: trạng thái sau khi xoá lùi 1 ký tự và sau khi
	// khôi phục phím gốc (Shift+Space trong bản IBus).
	AfterBackspace *string `json:"after_bs,omitempty"`
	AfterRestore   *string `json:"after_restore,omitempty"`
}

// Bảng chữ rút gọn: nguyên âm + phụ âm hay biến đổi + mọi phím dấu của các kiểu
// gõ (telex sfrxjzw d, vni 0-9, viqr ' ` ? ~ . ^ + ( -).
// Bảng chữ cho lượt vét cạn 3 phím (giữ nhỏ để bộ ca kiểm không phình).
const reducedAlphabet = "aeiouydswfrxjz123456'`?~.^+("

const shortAlphabet = "abcdefghijklmnopqrstuvwxyz0123456789'`?~.^+(-"

// LCG tự viết cho tái lập được y hệt ở mọi phiên bản Go.
type lcg uint64

func (r *lcg) next() uint64 {
	*r = lcg(uint64(*r)*6364136223846793005 + 1442695040888963407)
	return uint64(*r >> 16)
}

func (r *lcg) intn(n int) int { return int(r.next() % uint64(n)) }

func main() {
	var defs = bamboo.GetInputMethodDefinitions()
	var imNames []string
	for name := range defs {
		imNames = append(imNames, name)
	}
	sort.Strings(imNames)

	var out = bufio.NewWriterSize(os.Stdout, 1<<20)
	defer out.Flush()
	var enc = json.NewEncoder(out)

	var total int
	emit := func(c Case) {
		if err := enc.Encode(c); err != nil {
			fmt.Fprintln(os.Stderr, err)
			os.Exit(1)
		}
		total++
	}

	for _, im := range imNames {
		var method = bamboo.ParseInputMethod(defs, im)

		// 1) Ca kiểm tay: những chỗ bộ gõ hay sai (dấu lặp, huỷ dấu, ư/ơ, đ,
		//    "gi", "qu", chữ hoa, khôi phục phím gốc).
		for _, keys := range curatedCases {
			emit(run(im, method, bamboo.EstdFlags, keys, true))
		}

		// 2) Vét cạn mọi chuỗi 1–2 phím trên bảng chữ đầy đủ.
		for _, a := range shortAlphabet {
			emit(run(im, method, bamboo.EstdFlags, string(a), false))
			for _, b := range shortAlphabet {
				emit(run(im, method, bamboo.EstdFlags, string([]rune{a, b}), false))
			}
		}

		// 3) Vét cạn 3 phím trên bảng chữ rút gọn — chỉ cho ba họ kiểu gõ chính,
		//    để bộ ca kiểm không phình vô ích (các kiểu còn lại là biến thể).
		if im == "Telex" || im == "VNI" || im == "VIQR" {
			for _, a := range reducedAlphabet {
				for _, b := range reducedAlphabet {
					for _, c := range reducedAlphabet {
						emit(run(im, method, bamboo.EstdFlags, string([]rune{a, b, c}), false))
					}
				}
			}
		}

		// 4) Chuỗi dài 4–9 phím, sinh ngẫu nhiên nhưng CÓ HẠT GIỐNG CỐ ĐỊNH
		//    nên lần nào cũng ra đúng bộ đó.
		var rnd = lcg(20260809)
		for i := 0; i < 4000; i++ {
			var n = 4 + rnd.intn(6)
			var keys = make([]rune, n)
			for j := 0; j < n; j++ {
				keys[j] = rune(reducedAlphabet[rnd.intn(len(reducedAlphabet))])
			}
			emit(run(im, method, bamboo.EstdFlags, string(keys), false))
		}

		// 5) Đổi cờ: bỏ dấu tự do / kiểu dấu chuẩn / tự sửa lỗi bật-tắt.
		for _, fl := range []uint{0, bamboo.EfreeToneMarking, bamboo.EstdToneStyle,
			bamboo.EfreeToneMarking | bamboo.EstdToneStyle} {
			for _, keys := range curatedCases {
				emit(run(im, method, fl, keys, false))
			}
		}
	}

	fmt.Fprintf(os.Stderr, "đã sinh %d ca kiểm cho %d kiểu gõ\n", total, len(imNames))
}

func run(imName string, method bamboo.InputMethod, flags uint, keys string, withEdits bool) Case {
	var e = bamboo.NewEngine(method, flags)
	var steps []string
	for _, k := range keys {
		e.ProcessKey(k, bamboo.VietnameseMode)
		steps = append(steps, e.GetProcessedString(bamboo.VietnameseMode))
	}
	var c = Case{
		IM:    imName,
		Flags: flags,
		Keys:  keys,
		Steps: steps,
		Vi:    e.GetProcessedString(bamboo.VietnameseMode),
		Raw:   e.GetProcessedString(bamboo.EnglishMode | bamboo.FullText),
		Valid: e.IsValid(false),
	}
	if withEdits {
		e.RemoveLastChar(true)
		var bs = e.GetProcessedString(bamboo.VietnameseMode)
		c.AfterBackspace = &bs
		e.RestoreLastWord(false)
		var rs = e.GetProcessedString(bamboo.VietnameseMode)
		c.AfterRestore = &rs
	}
	return c
}
