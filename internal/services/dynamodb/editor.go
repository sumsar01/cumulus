package dynamodb

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/sumsar01/cumulus/internal/config"
)

// editorDoneMsg is emitted after the editor closes. Data is the parsed JSON
// bytes ready for PutItem; Err is non-nil if parsing failed or the file was
// unchanged.
type editorDoneMsg struct {
	Data []byte
	Err  error
}

// editorExec writes initialJSON to a temp file, execs the editor, reads the
// result, validates it as JSON, and returns an editorDoneMsg. It is designed
// to be used with tea.ExecProcess via a thin exec.Cmd wrapper.
//
// This function is the single point where the editor binary is executed.
func editorExec(cfg config.Config, initialJSON []byte) ([]byte, error) {
	// 1. Create a randomised temp directory (not a predictable path).
	dir, err := os.MkdirTemp("", "cumulus-*")
	if err != nil {
		return nil, fmt.Errorf("creating temp dir: %w", err)
	}
	defer os.RemoveAll(dir) // always clean up, even on error // #nosec G defer

	tmpFile := filepath.Join(dir, "item.json")

	// 2. Write with mode 0600 — owner read/write only.
	if err := os.WriteFile(tmpFile, initialJSON, 0600); err != nil { // #nosec G306
		return nil, fmt.Errorf("writing temp file: %w", err)
	}

	// 3. Exec editor directly — no shell, no shell expansion.
	//    cfg.Editor has already been validated against the allowlist by the
	//    config package at startup. We do a final sanity check here as defence
	//    in depth.
	editorBin, err := validateEditorBin(cfg.Editor)
	if err != nil {
		return nil, err
	}

	cmd := exec.Command(editorBin, tmpFile) // #nosec G204 — binary validated above
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr

	if err := cmd.Run(); err != nil {
		// Give a clear hint when the binary simply isn't in PATH.
		if errors.Is(err, exec.ErrNotFound) {
			// Build the allowed list dynamically from config.AllowedEditors.
			names := make([]string, 0, len(config.AllowedEditors))
			for k := range config.AllowedEditors {
				names = append(names, k)
			}
			sort.Strings(names)
			return nil, fmt.Errorf(
				"editor %q not found in PATH — install it or set a different editor in ~/.config/cumulus/config.toml\n(allowed: %s)",
				editorBin, strings.Join(names, ", "),
			)
		}
		return nil, fmt.Errorf("editor exited: %w", err)
	}

	// 4. Read the result back.
	data, err := os.ReadFile(tmpFile) // #nosec G304 — path is our own temp file
	if err != nil {
		return nil, fmt.Errorf("reading temp file after edit: %w", err)
	}

	// 5. Validate JSON before returning — never send unparseable bytes to the API.
	var v interface{}
	if err := json.Unmarshal(data, &v); err != nil {
		return nil, fmt.Errorf("result is not valid JSON: %w", err)
	}

	return data, nil
}

// startEditorCmd is the real entry point used by the items model. It runs
// editorExec in a goroutine and emits editorDoneMsg when done.
func startEditorCmd(cfg config.Config, initialJSON []byte) tea.Cmd {
	return func() tea.Msg {
		data, err := editorExec(cfg, initialJSON)
		return editorDoneMsg{Data: data, Err: err}
	}
}

// validateEditorBin checks the binary name against the hard-coded allowlist as
// a defence-in-depth measure. It returns the bare binary name (not a path) so
// that exec.Command uses PATH lookup rather than executing an arbitrary path.
func validateEditorBin(name string) (string, error) {
	if _, ok := config.AllowedEditors[name]; !ok {
		return "", fmt.Errorf("editor %q is not permitted", name)
	}
	return name, nil
}

// emptyItemJSON returns a minimal DynamoDB item JSON template for new items.
func emptyItemJSON() []byte {
	return []byte("{\n  \n}\n")
}
