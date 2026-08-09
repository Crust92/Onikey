/*
 * Onikey - đọc và đối chiếu bộ ca kiểm lõi tiếng Việt
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

// Package corpus đọc bộ ca kiểm đối chiếu và so nó với một bản cài đặt lõi
// tiếng Việt. Bộ ca kiểm sinh từ bản Go hiện tại (tools/gen-corpus) và là
// LƯỚI AN TOÀN cho việc viết lại lõi bằng Rust: bản mới phải khớp TỪNG PHÍM.
package corpus

import (
	"bufio"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"

	bamboo "github.com/BambooEngine/bamboo-core"
)

// Path là nơi đặt bộ ca kiểm, tính từ gốc kho.
const Path = "tests/corpus/core.jsonl.gz"

type Case struct {
	IM             string   `json:"im"`
	Flags          uint     `json:"flags"`
	Keys           string   `json:"keys"`
	Steps          []string `json:"steps"`
	Vi             string   `json:"vi"`
	Raw            string   `json:"raw"`
	Valid          bool     `json:"valid"`
	AfterBackspace *string  `json:"after_bs,omitempty"`
	AfterRestore   *string  `json:"after_restore,omitempty"`
}

// Mismatch mô tả một ca kiểm không khớp.
type Mismatch struct {
	Case  Case
	Field string
	Want  string
	Got   string
}

func (m Mismatch) String() string {
	return fmt.Sprintf("[%s cờ=%d] phím %q: %s mong đợi %q, nhận %q",
		m.Case.IM, m.Case.Flags, m.Case.Keys, m.Field, m.Want, m.Got)
}

// Load đọc bộ ca kiểm (nhận cả .gz lẫn .jsonl thường).
func Load(path string) ([]Case, error) {
	f, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer f.Close()

	var r io.Reader = f
	if strings.HasSuffix(path, ".gz") {
		zr, err := gzip.NewReader(f)
		if err != nil {
			return nil, err
		}
		defer zr.Close()
		r = zr
	}

	var cases []Case
	var sc = bufio.NewScanner(r)
	sc.Buffer(make([]byte, 1<<20), 1<<20)
	for sc.Scan() {
		var line = sc.Bytes()
		if len(line) == 0 {
			continue
		}
		var c Case
		if err := json.Unmarshal(line, &c); err != nil {
			return nil, err
		}
		cases = append(cases, c)
	}
	return cases, sc.Err()
}

// CheckGo chạy lại toàn bộ ca kiểm bằng chính lõi Go và trả về các chỗ lệch.
// Dừng sau maxReport chỗ lệch đầu tiên (0 = báo hết).
func CheckGo(cases []Case, maxReport int) []Mismatch {
	var defs = bamboo.GetInputMethodDefinitions()
	var methods = map[string]bamboo.InputMethod{}
	var out []Mismatch

	for _, c := range cases {
		m, ok := methods[c.IM]
		if !ok {
			m = bamboo.ParseInputMethod(defs, c.IM)
			methods[c.IM] = m
		}
		var e = bamboo.NewEngine(m, c.Flags)
		var bad = func(field, want, got string) bool {
			out = append(out, Mismatch{Case: c, Field: field, Want: want, Got: got})
			return maxReport > 0 && len(out) >= maxReport
		}

		var stop bool
		var i int
		for _, k := range c.Keys {
			e.ProcessKey(k, bamboo.VietnameseMode)
			var got = e.GetProcessedString(bamboo.VietnameseMode)
			if i < len(c.Steps) && got != c.Steps[i] {
				if bad(fmt.Sprintf("bước %d", i+1), c.Steps[i], got) {
					stop = true
					break
				}
			}
			i++
		}
		if stop {
			return out
		}
		if got := e.GetProcessedString(bamboo.VietnameseMode); got != c.Vi {
			if bad("vi", c.Vi, got) {
				return out
			}
		}
		if got := e.GetProcessedString(bamboo.EnglishMode | bamboo.FullText); got != c.Raw {
			if bad("raw", c.Raw, got) {
				return out
			}
		}
		if got := e.IsValid(false); got != c.Valid {
			if bad("valid", fmt.Sprint(c.Valid), fmt.Sprint(got)) {
				return out
			}
		}
		if c.AfterBackspace != nil {
			e.RemoveLastChar(true)
			if got := e.GetProcessedString(bamboo.VietnameseMode); got != *c.AfterBackspace {
				if bad("after_bs", *c.AfterBackspace, got) {
					return out
				}
			}
			if c.AfterRestore != nil {
				e.RestoreLastWord(false)
				if got := e.GetProcessedString(bamboo.VietnameseMode); got != *c.AfterRestore {
					if bad("after_restore", *c.AfterRestore, got) {
						return out
					}
				}
			}
		}
	}
	return out
}
