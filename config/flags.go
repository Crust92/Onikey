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
	// Bit này từng là "không gạch chân riêng ở ô địa chỉ trình duyệt". Đã bỏ:
	// IBnoUnderline làm được cho MỌI ô nhập nên tính năng riêng cho ô địa chỉ
	// chỉ còn là chỗ phức tạp thừa. Giữ chỗ để bit sau không bị dùng lại, kẻo
	// cấu hình cũ (đã bật bit này) tự dưng bật nhầm tính năng khác.
	_IBnoUnderlineForURL //deprecated
	IBstdFlags           = IBspellCheckEnabled | IBspellCheckWithRules | IBautoNonVnRestore | IBddFreeStyle |
		IBautoCapitalizeMacro | IBnoUnderline | IBworkaroundForWPS
	IBUsStdFlags = 0
)
