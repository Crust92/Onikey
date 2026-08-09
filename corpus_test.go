package main

import (
	"testing"

	"onikey/corpus"
)

// TestCorpus khoá hành vi lõi tiếng Việt: bộ ca kiểm sinh từ chính bản Go này
// (tools/gen-corpus) phải chạy lại ra kết quả y hệt. Nó vừa canh cho bản Go
// khỏi trôi khi nâng bamboo-core, vừa là thước đo cho lõi Rust sắp viết.
func TestCorpus(t *testing.T) {
	cases, err := corpus.Load(corpus.Path)
	if err != nil {
		t.Skipf("không có bộ ca kiểm (%v) — sinh bằng: go run ./tools/gen-corpus | gzip -n > %s", err, corpus.Path)
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
