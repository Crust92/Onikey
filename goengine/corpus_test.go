package main

import (
	"os"
	"path/filepath"
	"testing"

	"onikey/corpus"
)

// TestCorpus khoá hành vi lõi tiếng Việt: bộ ca kiểm sinh từ chính bản Go này
// (tools/gen-corpus) phải chạy lại ra kết quả y hệt. Nó vừa canh cho bản Go
// khỏi trôi khi nâng bamboo-core, vừa là thước đo cho lõi Rust sắp viết.
func TestCorpus(t *testing.T) {
	path := corpus.Path
	if _, statErr := os.Stat(path); statErr != nil {
		// Go chạy test trong thư mục của gói (./goengine), còn bộ ca kiểm nằm
		// ở gốc kho — ngó lên một cấp trước khi kết luận là không có.
		path = filepath.Join("..", path)
	}
	cases, err := corpus.Load(path)
	if err != nil {
		t.Skipf("không có bộ ca kiểm (%v) — sinh bằng: go run ./tools/gen-corpus | gzip -n > %s", err, path)
	}
	if len(cases) < 1000 {
		t.Fatalf("bộ ca kiểm quá nhỏ: %d ca", len(cases))
	}

	var mismatches = corpus.CheckGo(cases, 10)
	for _, m := range mismatches {
		t.Error(m)
	}
	if len(mismatches) > 0 {
		t.Fatalf("%d chỗ lệch so với bộ ca kiểm (%d ca kiểm)", len(mismatches), len(cases))
	}
	t.Logf("khớp %d ca kiểm", len(cases))
}
