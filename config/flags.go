package config

const (
	PreeditIM = iota + 1
	SurroundingTextIM
	BackspaceForwardingIM
	ShiftLeftForwardingIM
	ForwardAsCommitIM
	XTestFakeKeyEventIM
	UsIM
)

var ImLookupTable = map[int]string{
	PreeditIM:             "Cấu hình mặc định (Pre-edit)",
	SurroundingTextIM:     "Sửa lỗi gạch chân (Surrounding Text)",
	BackspaceForwardingIM: "Sửa lỗi gạch chân (ForwardKeyEvent I)",
	ShiftLeftForwardingIM: "Sửa lỗi gạch chân (ForwardKeyEvent II)",
	ForwardAsCommitIM:     "Sửa lỗi gạch chân (Forward as commit)",
	XTestFakeKeyEventIM:   "Sửa lỗi gạch chân (XTestFakeKeyEvent)",
	UsIM:                  "Thêm vào danh sách loại trừ",
}

var ImBackspaceList = []int{
	SurroundingTextIM,
	BackspaceForwardingIM,
	ShiftLeftForwardingIM,
	ForwardAsCommitIM,
	XTestFakeKeyEventIM,
}

const (
	IBautoCommitWithVnNotMatch uint = 1 << iota
	IBmacroEnabled
	_IBautoCommitWithVnFullMatch //deprecated
	_IBautoCommitWithVnWordBreak //deprecated
	IBspellCheckEnabled
	IBautoNonVnRestore
	IBddFreeStyle
	IBnoUnderline
	IBspellCheckWithRules
	IBspellCheckWithDicts
	IBautoCommitWithDelay
	_IBautoCommitWithMouseMovement //deprecated
	_IBemojiDisabled               //deprecated
	IBpreeditElimination
	_IBinputModeLookupTableEnabled //deprecated
	IBautoCapitalizeMacro
	_IBimQuickSwitchEnabled     //deprecated
	_IBrestoreKeyStrokesEnabled //deprecated
	_IBmouseCapturing           //deprecated
	IBworkaroundForFBMessenger
	IBworkaroundForWPS
	// Onikey: ô địa chỉ trình duyệt (content purpose = URL) luôn gõ ở chế độ
	// không gạch chân, kể cả khi chế độ mặc định là Pre-edit — vì pre-edit làm
	// hỏng danh sách gợi ý của thanh địa chỉ.
	IBnoUnderlineForURL
	IBstdFlags = IBspellCheckEnabled | IBspellCheckWithRules | IBautoNonVnRestore | IBddFreeStyle |
		IBautoCapitalizeMacro | IBnoUnderline | IBworkaroundForWPS | IBnoUnderlineForURL
	IBUsStdFlags = 0
)
