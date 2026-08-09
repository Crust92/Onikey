/*
 * Onikey - fork của ibus-bamboo
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

package main

import (
	"log"
	"os"
	"os/exec"
	"path/filepath"
)

const configBinName = "onikey-config"

// openConfigGUI mở hộp thoại cấu hình như MỘT TIẾN TRÌNH RIÊNG rồi chờ nó đóng
// (chờ để engine nạp lại cấu hình ngay sau đó). Engine không còn liên kết GTK,
// nên hộp thoại lỗi/panic cũng không làm mất gõ toàn hệ thống như trước.
func openConfigGUI(engineName string) {
	var bin = findConfigBin()
	var cmd = exec.Command(bin, "-engine", engineName)
	cmd.Stdout, cmd.Stderr = os.Stdout, os.Stderr
	if err := cmd.Run(); err != nil {
		log.Printf("không mở được hộp thoại cấu hình (%s): %v", bin, err)
	}
}

// findConfigBin ưu tiên bản nằm cạnh engine (cùng thư mục cài, kể cả khi cài
// vào prefix lạ), sau đó mới tìm trong PATH.
func findConfigBin() string {
	if self, err := os.Executable(); err == nil {
		var sibling = filepath.Join(filepath.Dir(self), configBinName)
		if st, err := os.Stat(sibling); err == nil && !st.IsDir() {
			return sibling
		}
	}
	if p, err := exec.LookPath(configBinName); err == nil {
		return p
	}
	return configBinName
}
