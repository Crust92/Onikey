package config

import (
	"encoding/json"
	"fmt"
	"io/ioutil"
	"log"
	"os"
	"os/user"
	"path/filepath"

	"github.com/BambooEngine/bamboo-core"
)

const (
	configDir        = "%s/.config/%s"
	configFile       = "%s/%s.config.json"
	mactabFile       = "%s/%s.macro.text"
	sampleMactabFile = "data/macro.tpl.txt"

	// Thư mục cấu hình thời còn mang tên ibus-bamboo; giữ để chuyển cấu hình cũ
	// sang tên mới đúng một lần (xem MigrateLegacyConfig).
	legacyConfigDir  = "%s/.config/ibus-bamboo"
	legacyConfigFile = "ibus-bamboo.config.json"
	legacyMacroFile  = "ibus-bamboo.macro.text"
)

// EngineID là tên dùng cho thư mục/ tệp cấu hình khi engine chạy ở nhánh mặc
// định. Các biến thể (Onikey::Us…) vẫn truyền tên riêng của chúng.
const EngineID = "onikey"

type Config struct {
	InputMethod            string
	InputMethodDefinitions map[string]bamboo.InputMethodDefinition
	OutputCharset          string
	Flags                  uint
	IBflags                uint
	Shortcuts              [10]uint32
	DefaultInputMode       int
	InputModeMapping       map[string]int
}

func GetConfigDir(ngName string) string {
	u, err := user.Current()
	if err == nil {
		return fmt.Sprintf(configDir, u.HomeDir, EngineID)
	}
	return fmt.Sprintf(configDir, "~", EngineID)
}

// MigrateLegacyConfig chép cấu hình từ thời tên cũ (~/.config/ibus-bamboo) sang
// thư mục mới nếu thư mục mới chưa có gì — để người đang dùng bản cũ nâng cấp
// mà không mất kiểu gõ, gõ tắt, phím tắt. Chỉ chép, KHÔNG xóa bản cũ.
func MigrateLegacyConfig(engineName string) {
	u, err := user.Current()
	if err != nil {
		return
	}
	var oldDir = fmt.Sprintf(legacyConfigDir, u.HomeDir)
	var newDir = GetConfigDir(engineName)
	if oldDir == newDir {
		return
	}
	var pairs = [][2]string{
		{filepath.Join(oldDir, legacyConfigFile), GetConfigPath(engineName)},
		{filepath.Join(oldDir, legacyMacroFile), GetMacroPath(engineName)},
	}
	for _, p := range pairs {
		if _, err := os.Stat(p[1]); err == nil {
			continue // đã có bản mới thì không đụng tới
		}
		data, err := ioutil.ReadFile(p[0])
		if err != nil {
			continue
		}
		if err := os.MkdirAll(newDir, 0755); err != nil {
			return
		}
		if err := ioutil.WriteFile(p[1], data, 0644); err != nil {
			log.Println(err)
			continue
		}
		log.Printf("Đã chuyển cấu hình cũ: %s -> %s", p[0], p[1])
	}
}

func GetMacroPath(engineName string) string {
	return fmt.Sprintf(mactabFile, GetConfigDir(engineName), engineName)
}

func GetConfigPath(engineName string) string {
	return fmt.Sprintf(configFile, GetConfigDir(engineName), engineName)
}

func DefaultCfg() Config {
	return Config{
		InputMethod:            "Telex 2",
		OutputCharset:          "Unicode",
		InputMethodDefinitions: bamboo.GetInputMethodDefinitions(),
		Flags:                  bamboo.EstdFlags,
		IBflags:                IBstdFlags,
		Shortcuts:              [10]uint32{1, 126, 0, 0, 0, 0, 0, 0, 5, 117},
		// Onikey: mặc định Pre-edit (có gạch chân) vì tin cậy nhất khi máy lag.
		// Muốn gõ không gạch chân thì bật cờ IBnoUnderline — engine sẽ chuyển
		// sang Surrounding Text ở những app hỗ trợ (xem updateNoUnderlineMode).
		DefaultInputMode: PreeditIM,
		InputModeMapping: map[string]int{},
	}
}

func LoadConfig(engineName string) *Config {
	var c = DefaultCfg()
	if engineName == "onikeyus" {
		c.DefaultInputMode = UsIM
		c.IBflags = IBUsStdFlags
		return &c
	}

	data, err := ioutil.ReadFile(GetConfigPath(engineName))
	if err == nil {
		json.Unmarshal(data, &c)
	}

	return &c
}

func SaveConfig(c *Config, engineName string) {
	data, err := json.MarshalIndent(c, "", "  ")
	if err != nil {
		return
	}

	err = ioutil.WriteFile(fmt.Sprintf(configFile, GetConfigDir(engineName), engineName), data, 0644)
	if err != nil {
		log.Println(err)
	}

}
