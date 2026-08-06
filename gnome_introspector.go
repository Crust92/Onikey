package main

import (
	"context"
	"errors"
	"sync/atomic"
	"time"

	"github.com/godbus/dbus/v5"
)

// GNOME 41 trở lên KHÓA org.gnome.Shell.Eval (trừ khi bật unsafe-mode): gọi bao
// nhiêu lần cũng chỉ nhận về (false, "") mà vẫn tốn round-trip DBus ĐỒNG BỘ
// ngay trong FocusIn. Lúc gnome-shell bận (ví dụ đang mở Edge) round-trip đó
// kéo dài, làm nghẽn xử lý focus của bộ gõ -> app treo con trỏ quay tròn cho
// tới khi đổi sang bộ gõ khác. Nên: dò một lần, thấy bị khóa thì thôi hẳn, và
// mọi lời gọi đều có hạn chờ.
var gnomeEvalBlocked int32

var errGnomeEvalBlocked = errors.New("org.gnome.Shell.Eval bị khóa")

const gnomeEvalTimeout = 300 * time.Millisecond

func gnomeGetFocusWindowClass() (string, error) {
	if atomic.LoadInt32(&gnomeEvalBlocked) != 0 {
		return "", errGnomeEvalBlocked
	}
	conn, err := dbus.SessionBus()
	if err != nil {
		return "", err
	}
	defer func() {
		if err = conn.Hello(); err == nil {
			conn.Close()
		}
	}()

	js_code := "global.get_window_actors().find(window => !Main.overview.visible && window.meta_window.has_focus()).get_meta_window().get_wm_class()"
	ok, s, err := gnomeShellEval(conn, js_code)
	if err != nil {
		return "", err
	}
	if !ok {
		if isGnomeOverviewVisible(conn) {
			return "org.gnome.Overview", nil
		}
		return "", errors.New(s)
	}
	return s, nil
}

func isGnomeOverviewVisible(conn *dbus.Conn) bool {
	ok, visible, err := gnomeShellEval(conn, "Main.overview.visible")
	if !ok || err != nil {
		return false
	}
	return visible == "true"
}

// gnomeShellEval gọi Shell.Eval có hạn chờ. Eval trả về (false, "") nghĩa là
// shell không hề chạy đoạn JS (bị khóa) — khác với JS chạy mà lỗi, khi đó chuỗi
// trả về có nội dung lỗi. Bị khóa hoặc lỗi DBus thì ghim lại để lần sau khỏi gọi.
func gnomeShellEval(conn *dbus.Conn, jsCode string) (bool, string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), gnomeEvalTimeout)
	defer cancel()

	var ok bool
	var out string
	obj := conn.Object("org.gnome.Shell", "/org/gnome/Shell")
	err := obj.CallWithContext(ctx, "org.gnome.Shell.Eval", 0, jsCode).Store(&ok, &out)
	if err != nil || (!ok && out == "") {
		atomic.StoreInt32(&gnomeEvalBlocked, 1)
		dbg("gnomeShellEval: tắt hẳn dò WM_CLASS qua Shell.Eval (ok=%v out=%q err=%v)", ok, out, err)
	}
	return ok, out, err
}
