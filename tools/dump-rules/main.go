/*
 * Onikey - đổ bảng luật gõ đã phân tích ra JSON để đối chiếu với bản Rust
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

// dump-rules in ra bảng luật mà bản Go phân tích được từ định nghĩa kiểu gõ.
// Bản Rust phải phân tích ra ĐÚNG bảng này — đây là mốc đầu tiên của cuộc port,
// vì phần DSL ("A_Â", "UOA_ƯƠĂ__Ư", "__ư") rất dễ hiểu sai.
//
//	go run ./tools/dump-rules > tests/corpus/rules.json
//
// Luật được GOM THEO PHÍM và sắp xếp, vì bản Go duyệt map nên thứ tự giữa các
// phím vốn không ổn định — chỉ thứ tự TRONG một phím mới có ý nghĩa.
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"sort"

	bamboo "github.com/BambooEngine/bamboo-core"
)

type jsonRule struct {
	Effect     uint8    `json:"effect"`
	EffectType int      `json:"effect_type"`
	EffectOn   string   `json:"effect_on"`
	Result     string   `json:"result"`
	Appended   []string `json:"appended,omitempty"`
}

type jsonIM struct {
	Keys          []string              `json:"keys"`
	SuperKeys     []string              `json:"super_keys"`
	ToneKeys      []string              `json:"tone_keys"`
	AppendingKeys []string              `json:"appending_keys"`
	RulesByKey    map[string][]jsonRule `json:"rules_by_key"`
}

func runesToStrings(rs []rune) []string {
	var out = make([]string, 0, len(rs))
	for _, r := range rs {
		out = append(out, string(r))
	}
	sort.Strings(out)
	return out
}

func main() {
	var defs = bamboo.GetInputMethodDefinitions()
	var names []string
	for n := range defs {
		names = append(names, n)
	}
	sort.Strings(names)

	var out = map[string]jsonIM{}
	for _, name := range names {
		var im = bamboo.ParseInputMethod(defs, name)
		var entry = jsonIM{
			Keys:          runesToStrings(im.Keys),
			SuperKeys:     runesToStrings(im.SuperKeys),
			ToneKeys:      runesToStrings(im.ToneKeys),
			AppendingKeys: runesToStrings(im.AppendingKeys),
			RulesByKey:    map[string][]jsonRule{},
		}
		for _, r := range im.Rules {
			var jr = jsonRule{
				Effect:     r.Effect,
				EffectType: int(r.EffectType),
				EffectOn:   string(r.EffectOn),
				Result:     string(r.Result),
			}
			for _, ar := range r.AppendedRules {
				jr.Appended = append(jr.Appended, string(ar.Result))
			}
			var k = string(r.Key)
			entry.RulesByKey[k] = append(entry.RulesByKey[k], jr)
		}
		out[name] = entry
	}

	var enc = json.NewEncoder(os.Stdout)
	enc.SetIndent("", " ")
	if err := enc.Encode(out); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
