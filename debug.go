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
	"path/filepath"
	"sync"

	"onikey/config"
)

// Engine do ibus-daemon khởi chạy nên stdout/stderr thường không xem được.
// Bật log gỡ rối bằng cách tạo file cờ:
//
//	touch ~/.config/onikey/onikey-debug
//
// rồi `ibus restart`. Log ghi vào ~/.config/onikey/onikey-debug.log.
// Không có file cờ thì hàm dbg() không làm gì cả (không tốn I/O).
const (
	debugFlagFile = "onikey-debug"
	debugLogFile  = "onikey-debug.log"
)

var (
	dbgOnce   sync.Once
	dbgLogger *log.Logger
)

func dbg(format string, args ...interface{}) {
	dbgOnce.Do(initDbgLogger)
	if dbgLogger == nil {
		return
	}
	dbgLogger.Printf(format, args...)
}

func initDbgLogger() {
	var dir = config.GetConfigDir("onikey")
	if _, err := os.Stat(filepath.Join(dir, debugFlagFile)); err != nil {
		return
	}
	f, err := os.OpenFile(filepath.Join(dir, debugLogFile),
		os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		return
	}
	dbgLogger = log.New(f, "", log.LstdFlags|log.Lmicroseconds)
	dbgLogger.Printf("=== onikey debug log bắt đầu (pid %d, version %s) ===", os.Getpid(), Version)
}
