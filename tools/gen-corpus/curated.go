package main

// curatedCases là các chuỗi phím ĐÁNG NGỜ NHẤT — chỗ mà một bản cài đặt mới rất
// dễ làm sai mà kiểm thử ngẫu nhiên khó chạm tới. Cùng một chuỗi được chạy qua
// mọi kiểu gõ: với kiểu không dùng phím đó thì kết quả "không biến đổi" cũng là
// một hành vi cần khoá lại.
var curatedCases = []string{
	// nguyên âm đôi -> mũ/móc, và gõ lặp để huỷ
	"aa", "aaa", "aaaa", "ee", "eee", "oo", "ooo",
	"aw", "aww", "ow", "oww", "uw", "uww", "w", "ww", "www",
	"dd", "ddd", "dddd",

	// dấu thanh: đặt, đổi, huỷ bằng cách gõ lại, và phím z xoá dấu
	"as", "af", "ar", "ax", "aj", "az",
	"ass", "asf", "asz", "aszs",
	"as1", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a0",

	// bỏ dấu tự do: dấu gõ sớm, gõ giữa, gõ muộn
	"tieengs", "tiengs", "tiesng", "tsieng", "tieesng",
	"hoas", "hoaas", "hosa", "hoafng", "hoangf",

	// ư/ơ và tổ hợp ươ
	"uwown", "uowng", "duwowngj", "dduowngf", "nuowcs", "nuwowcs",
	"tuw", "tuwr", "thuw", "thuwr", "chuwa", "chuwaa",

	// "gi", "qu" — hai chỗ luật đặt dấu khác thường
	"gi", "gii", "gia", "gias", "giaf", "gio", "gios", "giuw",
	"qu", "qua", "quas", "quaa", "quaas", "quyn", "quyeenr", "quawng",

	// vần khó, phụ âm cuối, âm đệm
	"nghieengs", "nghieeng", "khuyeen", "khuyeenr", "thuyeenf",
	"ngoaif", "hoawcj", "khoainr", "chuyeenj", "nguyeenx",

	// chữ hoa và hoa toàn phần
	"Tieengs", "TIEENGS", "Vieejt", "VIEEJT", "DDaay", "DDAAY",
	"Aa", "AA", "Ww", "WW", "Dd", "DD",

	// tiếng Anh lọt vào (phải giữ nguyên hoặc khôi phục được)
	"the", "there", "email", "password", "google", "linux",
	"www", "http", "css", "java", "queue", "would",

	// phím không phải chữ xen vào
	"a1b", "a.b", "a-b", "a_b", "a b", "a,b", "a;b",

	// chuỗi dài hơn một tiếng
	"tieengs Vieejt", "chungs toi", "ddi hocj", "ban laf ai",
	"cams own banj", "hejn gawpj laij",

	// VNI / VIQR: cùng ý nghĩa nhưng bằng số và ký hiệu
	"tie61ng", "vie65t", "d9a6y", "ti1nh", "quye6n2",
	"tie^'ng", "vie^.t", "d-a^y", "ti'nh", "quye^`n",
	"a('", "a^'", "o+'", "u+'", "d-", "d-d-",
}
