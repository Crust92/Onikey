/*
 * Bamboo - A Vietnamese Input method editor
 * Copyright (C) 2018 Luong Thanh Lam <ltlam93@gmail.com>
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
	"fmt"
	"log"
	"os/exec"
	"reflect"
	"strconv"
	"sync"
	"sync/atomic"
	"time"

	"github.com/BambooEngine/bamboo-core"
	ibus "github.com/BambooEngine/goibus"
	"github.com/godbus/dbus/v5"

	"onikey/config"
)

type OnikeyEngine struct {
	sync.Mutex
	IEngine
	preeditor            bamboo.IEngine
	engineName           string
	config               *config.Config
	propList             *ibus.PropList
	englishMode          bool
	macroTable           *MacroTable
	wmClasses            string
	isInputModeLTOpened  bool
	isEmojiLTOpened      bool
	isInHexadecimal      bool
	emojiLookupTable     *ibus.LookupTable
	inputModeLookupTable *ibus.LookupTable
	capabilities         uint32
	keyPressDelay        int
	// Kiểu nội dung của ô nhập hiện tại, do client báo qua IBus SetContentType
	// (purpose theo IBusInputPurpose: URL / email / password...).
	contentPurpose uint32
	contentHints   uint32
	// Chế độ gõ chốt cho ô địa chỉ (0 = ô hiện tại không phải ô địa chỉ).
	urlInputMode           int
	nFakeBackSpace         int32
	isFirstTimeSendingBS   bool
	emoji                  *EmojiEngine
	isSurroundingTextReady bool
	lastKeyWithShift       bool
	lastCommitText         int64
	// restore key strokes by pressing Shift + Space
	shouldRestoreKeyStrokes bool
	// enqueue key strokes to process later
	shouldEnqueuKeyStrokes bool
	// event-based confirmation for SurroundingText mode: after
	// DeleteSurroundingText we ask the app for the updated surrounding text
	// and wait for its reply before committing (instead of a blind fixed
	// sleep), so tone correctness adapts to app/system lag.
	stConfirmCh       chan struct{}
	awaitingSTConfirm int32
	stConfirmTimeouts int32
}

func NewOnikeyEngine(name string, cfg *config.Config, base IEngine, preeditor bamboo.IEngine) *OnikeyEngine {
	return &OnikeyEngine{
		engineName:  name,
		IEngine:     base,
		preeditor:   preeditor,
		config:      cfg,
		stConfirmCh: make(chan struct{}, 1),
	}
}

/*
*
Implement IBus.Engine's process_key_event default signal handler.

Args:

	keyval - The keycode, transformed through a keymap, stays the
		same for every keyboard
	keycode - Keyboard-dependant key code
	modifiers - The state of IBus.ModifierType keys like
		Shift, Control, etc.

Return:

	True - if successfully process the keyevent
	False - otherwise. The keyevent will be passed to X-Client

This function gets called whenever a key is pressed.
*/
func (e *OnikeyEngine) ProcessKeyEvent(keyVal uint32, keyCode uint32, state uint32) (bool, *dbus.Error) {
	if state&IBusReleaseMask != 0 {
		// fmt.Println("Ignore key-up event")
		return false, nil
	}
	fmt.Printf("\n")
	log.Printf(">>>>ProcessKeyEvent >  %d | state %d keyVal 0x%04x | %c <<<<\n", len(keyPressChan), state, keyVal, rune(keyVal))
	if ret, retValue := e.processShortcutKey(keyVal, keyCode, state); ret {
		return retValue, nil
	}
	if e.inBackspaceWhiteList() {
		return e.bsProcessKeyEvent(keyVal, keyCode, state)
	}
	return e.preeditProcessKeyEvent(keyVal, keyCode, state)
}

func (e *OnikeyEngine) FocusIn() *dbus.Error {
	log.Print("FocusIn.")
	var start = time.Now()
	var latestWm = e.getLatestWmClass()
	e.checkWmClass(latestWm)
	// Chốt lại chế độ ngay lúc focus: capability và kiểu ô nhập tới rải rác từ
	// nhiều input context, tới đây mới là trạng thái của ô đang thực sự gõ.
	e.updateURLInputMode()
	e.RegisterProperties(e.propList)
	e.RequireSurroundingText()
	if e.isShortcutKeyEnable(KSEmojiDialog) && emojiTrie != nil && len(emojiTrie.Children) == 0 {
		var err error
		emojiTrie, err = loadEmojiOne(DictEmojiOne)
		if err != nil {
			panic(fmt.Sprintf("failed to load emojiTrie from %s: %s", DictEmojiOne, err))
		}
	}
	if e.config.IBflags&config.IBspellCheckWithDicts != 0 && len(dictionary) == 0 {
		dictionary, _ = loadDictionary(DictVietnameseCm)
	}
	fmt.Printf("WM_CLASS=(%s)\n", e.getWmClass())
	dbg("FocusIn: wm=%q cap=0x%x purpose=%d hints=0x%x inputMode=%d took=%s",
		e.getWmClass(), e.capabilities, e.contentPurpose, e.contentHints, e.getInputMode(), time.Since(start))
	return nil
}

func (e *OnikeyEngine) FocusOut() *dbus.Error {
	log.Print("FocusOut.")
	dbg("FocusOut: purpose=%d hints=0x%x cap=0x%x", e.contentPurpose, e.contentHints, e.capabilities)
	return nil
}

func (e *OnikeyEngine) Reset() *dbus.Error {
	fmt.Print("Reset.\n")
	if e.checkInputMode(config.PreeditIM) {
		e.preeditor.Reset()
	}
	return nil
}

func (e *OnikeyEngine) Enable() *dbus.Error {
	fmt.Print("Enable.")
	e.RequireSurroundingText()
	return nil
}

func (e *OnikeyEngine) Disable() *dbus.Error {
	fmt.Print("Disable.")
	return nil
}

// @method(in_signature="vuu")
func (e *OnikeyEngine) SetSurroundingText(text dbus.Variant, cursorPos uint32, anchorPos uint32) *dbus.Error {
	if atomic.CompareAndSwapInt32(&e.awaitingSTConfirm, 1, 0) {
		// This callback is the app acknowledging our DeleteSurroundingText.
		// Signal the waiter and skip the buffer repopulation below (it would
		// clobber the in-progress composition).
		select {
		case e.stConfirmCh <- struct{}{}:
		default:
		}
		return nil
	}
	if !e.isSurroundingTextReady {
		//fmt.Println("Surrounding Text is not ready yet.")
		return nil
	}
	e.Lock()
	defer func() {
		e.Unlock()
		e.isSurroundingTextReady = false
		if err := recover(); err != nil {
			fmt.Println(err)
		}
	}()
	if e.inBackspaceWhiteList() {
		var str = reflect.ValueOf(reflect.ValueOf(text.Value()).Index(2).Interface()).String()
		var s = []rune(str)
		if len(s) < int(cursorPos) {
			return nil
		}
		var cs = s[:cursorPos]
		fmt.Println("Surrounding Text: ", string(cs))
		e.preeditor.Reset()
		for i := len(cs) - 1; i >= 0; i-- {
			// workaround for spell checking
			if bamboo.IsPunctuationMark(cs[i]) && e.preeditor.CanProcessKey(cs[i]) {
				cs[i] = ' '
			}
			e.preeditor.ProcessKey(cs[i], bamboo.EnglishMode|bamboo.InReverseOrder)
		}
	}
	return nil
}

// waitForSurroundingTextSync asks the app for its (post-delete) surrounding
// text and blocks until the app answers or maxWait elapses. This replaces a
// blind fixed sleep between DeleteSurroundingText and CommitText, so the commit
// only fires after the app has actually applied the deletion — making tone
// correctness resilient to app/system lag. If the app repeatedly fails to
// answer, the wait is capped short so typing stays responsive.
func (e *OnikeyEngine) waitForSurroundingTextSync(maxWait time.Duration) {
	if e.stConfirmCh == nil {
		time.Sleep(45 * time.Millisecond)
		return
	}
	// drain any stale confirmation
	select {
	case <-e.stConfirmCh:
	default:
	}
	atomic.StoreInt32(&e.awaitingSTConfirm, 1)
	e.RequireSurroundingText()
	wait := maxWait
	if atomic.LoadInt32(&e.stConfirmTimeouts) >= 3 {
		// App doesn't seem to report surrounding text back; don't stall the
		// full maxWait on every correction.
		wait = 45 * time.Millisecond
	}
	select {
	case <-e.stConfirmCh:
		atomic.StoreInt32(&e.stConfirmTimeouts, 0)
	case <-time.After(wait):
		atomic.StoreInt32(&e.awaitingSTConfirm, 0)
		atomic.AddInt32(&e.stConfirmTimeouts, 1)
	}
}

func (e *OnikeyEngine) PageUp() *dbus.Error {
	if e.isEmojiLTOpened && e.emojiLookupTable.PageUp() {
		e.updateEmojiLookupTable()
	}
	if e.isInputModeLTOpened && e.inputModeLookupTable.PageUp() {
		e.updateInputModeLT()
	}
	return nil
}

func (e *OnikeyEngine) PageDown() *dbus.Error {
	if e.isEmojiLTOpened && e.emojiLookupTable.PageDown() {
		e.updateEmojiLookupTable()
	}
	if e.isInputModeLTOpened && e.inputModeLookupTable.PageDown() {
		e.updateInputModeLT()
	}
	return nil
}

func (e *OnikeyEngine) CursorUp() *dbus.Error {
	if e.isEmojiLTOpened && e.emojiLookupTable.CursorUp() {
		e.updateEmojiLookupTable()
	}
	if e.isInputModeLTOpened && e.inputModeLookupTable.CursorUp() {
		e.updateInputModeLT()
	}
	return nil
}

func (e *OnikeyEngine) CursorDown() *dbus.Error {
	if e.isEmojiLTOpened && e.emojiLookupTable.CursorDown() {
		e.updateEmojiLookupTable()
	}
	if e.isInputModeLTOpened && e.inputModeLookupTable.CursorDown() {
		e.updateInputModeLT()
	}
	return nil
}

func (e *OnikeyEngine) CandidateClicked(index uint32, button uint32, state uint32) *dbus.Error {
	if e.isEmojiLTOpened && e.updateCursorPosInEmojiTable(index) {
		e.commitEmojiCandidate()
		e.closeEmojiCandidates()
	}
	if e.isInputModeLTOpened && e.inputModeLookupTable.SetCursorPos(index) {
		e.commitInputModeCandidate()
		e.closeInputModeCandidates()
	}
	return nil
}

func (e *OnikeyEngine) SetCapabilities(cap uint32) *dbus.Error {
	dbg("SetCapabilities: cap=0x%x", cap)
	e.capabilities = cap
	e.updateURLInputMode()
	return nil
}

func (e *OnikeyEngine) SetCursorLocation(x int32, y int32, w int32, h int32) *dbus.Error {
	return nil
}

func (e *OnikeyEngine) SetContentType(purpose uint32, hints uint32) *dbus.Error {
	dbg("SetContentType: purpose=%d hints=0x%x wm=%q", purpose, hints, e.getWmClass())
	e.checkContentPurpose(purpose, hints)
	return nil
}

// Set nhận org.freedesktop.DBus.Properties.Set. Từ IBus 1.5, kiểu nội dung của
// ô nhập KHÔNG gửi qua phương thức SetContentType nữa mà gửi bằng thuộc tính
// DBus "ContentType" kiểu (uu) = (purpose, hints) — thiếu hàm này thì bộ gõ
// không bao giờ biết ô đang gõ là ô địa chỉ hay ô văn bản thường.
func (e *OnikeyEngine) Set(iface string, propName string, value dbus.Variant) *dbus.Error {
	dbg("Properties.Set: iface=%s prop=%s value=%v", iface, propName, value)
	if propName != "ContentType" {
		return nil
	}
	purpose, hints, ok := parseContentType(value)
	if !ok {
		return nil
	}
	dbg("ContentType: purpose=%d hints=0x%x wm=%q", purpose, hints, e.getWmClass())
	e.checkContentPurpose(purpose, hints)
	return nil
}

func parseContentType(value dbus.Variant) (uint32, uint32, bool) {
	fields, ok := value.Value().([]interface{})
	if !ok || len(fields) != 2 {
		return 0, 0, false
	}
	purpose, ok1 := fields[0].(uint32)
	hints, ok2 := fields[1].(uint32)
	if !ok1 || !ok2 {
		return 0, 0, false
	}
	return purpose, hints, true
}

// @method(in_signature="su")
func (e *OnikeyEngine) PropertyActivate(propName string, propState uint32) *dbus.Error {
	if propName == PropKeyAbout {
		exec.Command("xdg-open", HomePage).Start()
		return nil
	}
	if propName == PropKeyVnCharsetConvert {
		exec.Command("xdg-open", CharsetConvertPage).Start()
		return nil
	}
	if propName == PropKeyConfiguration {
		openConfigGUI(e.engineName)
		e.config = config.LoadConfig(e.engineName)
		return nil
	}
	if propName == PropKeyInputModeLookupTableShortcut {
		openConfigGUI(e.engineName)
		e.config = config.LoadConfig(e.engineName)
		return nil
	}
	if propName == PropKeyMacroTable {
		openConfigGUI(e.engineName)
		e.config = config.LoadConfig(e.engineName)
		return nil
	}

	turnSpellChecking := func(on bool) {
		if on {
			e.config.IBflags |= config.IBspellCheckEnabled
			e.config.IBflags |= config.IBautoNonVnRestore
			if e.config.IBflags&config.IBspellCheckWithDicts == 0 {
				e.config.IBflags |= config.IBspellCheckWithRules
			}
		} else {
			e.config.IBflags &= ^config.IBspellCheckEnabled
			e.config.IBflags &= ^config.IBautoNonVnRestore
		}
	}

	if propName == PropKeyStdToneStyle {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.Flags |= bamboo.EstdToneStyle
		} else {
			e.config.Flags &= ^bamboo.EstdToneStyle
		}
	}
	if propName == PropKeyFreeToneMarking {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.Flags |= bamboo.EfreeToneMarking
		} else {
			e.config.Flags &= ^bamboo.EfreeToneMarking
		}
	}
	if propName == PropKeyEnableSpellCheck {
		if propState == ibus.PROP_STATE_CHECKED {
			turnSpellChecking(true)
		} else {
			turnSpellChecking(false)
		}
	}
	if propName == PropKeySpellCheckByRules {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.IBflags |= config.IBspellCheckWithRules
			turnSpellChecking(true)
		} else {
			e.config.IBflags &= ^config.IBspellCheckWithRules
		}
	}
	if propName == PropKeySpellCheckByDicts {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.IBflags |= config.IBspellCheckWithDicts
			turnSpellChecking(true)
			dictionary, _ = loadDictionary(DictVietnameseCm)
		} else {
			e.config.IBflags &= ^config.IBspellCheckWithDicts
		}
	}
	if propName == PropKeyMacroEnabled {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.IBflags |= config.IBmacroEnabled
			e.macroTable.Enable(e.engineName)
		} else {
			e.config.IBflags &= ^config.IBmacroEnabled
			e.macroTable.Disable()
		}
	}
	if propName == PropKeyPreeditInvisibility {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.IBflags |= config.IBnoUnderline
		} else {
			e.config.IBflags &= ^config.IBnoUnderline
		}
	}
	if propName == PropKeyPreeditElimination {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.IBflags |= config.IBpreeditElimination
		} else {
			e.config.IBflags &= ^config.IBpreeditElimination
		}
	}
	if propName == PropKeyAutoCapitalizeMacro {
		if propState == ibus.PROP_STATE_CHECKED {
			e.config.IBflags |= config.IBautoCapitalizeMacro
		} else {
			e.config.IBflags &= ^config.IBautoCapitalizeMacro
		}
		if e.config.IBflags&config.IBmacroEnabled != 0 {
			e.macroTable.Reload(e.engineName, e.config.IBflags&config.IBautoCapitalizeMacro != 0)
		}
	}

	var im, foundIm = getValueFromPropKey(propName, "InputMode")
	if foundIm && propState == ibus.PROP_STATE_CHECKED {
		e.config.DefaultInputMode, _ = strconv.Atoi(im)
	}
	var charset, foundCs = getValueFromPropKey(propName, "OutputCharset")
	if foundCs && isValidCharset(charset) && propState == ibus.PROP_STATE_CHECKED {
		e.config.OutputCharset = charset
	}
	if _, found := e.config.InputMethodDefinitions[propName]; found && propState == ibus.PROP_STATE_CHECKED {
		e.config.InputMethod = propName
	}
	if propName != "-" {
		config.SaveConfig(e.config, e.engineName)
	}
	e.propList = GetPropListByConfig(e.config)

	var inputMethod = bamboo.ParseInputMethod(e.config.InputMethodDefinitions, e.config.InputMethod)
	e.preeditor = bamboo.NewEngine(inputMethod, e.config.Flags)
	e.RegisterProperties(e.propList)
	return nil
}
