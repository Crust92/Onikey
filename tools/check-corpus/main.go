/*
 * Onikey - đối chiếu một bản cài đặt lõi với bộ ca kiểm
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

// check-corpus chạy lại bộ ca kiểm bằng lõi Go và báo mọi chỗ lệch.
//
//	go run ./tools/check-corpus [đường/dẫn/core.jsonl.gz]
//
// Khi lõi Rust ra đời, nó phải vượt qua ĐÚNG bộ ca kiểm này với cùng cách so
// sánh (từng phím, rồi vi/raw/valid, rồi xoá lùi và khôi phục phím gốc).
package main

import (
	"fmt"
	"os"

	"onikey/corpus"
)

func main() {
	var path = corpus.Path
	if len(os.Args) > 1 {
		path = os.Args[1]
	}

	cases, err := corpus.Load(path)
	if err != nil {
		fmt.Fprintln(os.Stderr, "không đọc được bộ ca kiểm:", err)
		os.Exit(2)
	}

	var mismatches = corpus.CheckGo(cases, 20)
	fmt.Printf("đã đối chiếu %d ca kiểm\n", len(cases))
	if len(mismatches) == 0 {
		fmt.Println("khớp hoàn toàn")
		return
	}
	for _, m := range mismatches {
		fmt.Println(m)
	}
	fmt.Printf("LỆCH: %d chỗ (báo tối đa 20)\n", len(mismatches))
	os.Exit(1)
}
