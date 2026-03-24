package dynamodb

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"time"

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

// drainExecCommand wraps an *exec.Cmd and implements tea.ExecCommand.
// After Run() returns it drains any pending bytes from stdin (with a short
// deadline) so that terminal capability query responses emitted by the editor
// (e.g. vim's DA2 / t_RV queries) do not leak into Bubble Tea's input loop
// or the parent shell once the TUI exits.
type drainExecCommand struct {
	cmd *exec.Cmd
	// stdin/stdout/stderr are set by Bubble Tea via the ExecCommand interface.
	stdin  io.Reader
	stdout io.Writer
	stderr io.Writer
}

func (d *drainExecCommand) SetStdin(r io.Reader)  { d.stdin = r }
func (d *drainExecCommand) SetStdout(w io.Writer) { d.stdout = w }
func (d *drainExecCommand) SetStderr(w io.Writer) { d.stderr = w }

func (d *drainExecCommand) Run() error {
	if d.cmd.Stdin == nil {
		d.cmd.Stdin = d.stdin
	}
	if d.cmd.Stdout == nil {
		d.cmd.Stdout = d.stdout
	}
	if d.cmd.Stderr == nil {
		d.cmd.Stderr = d.stderr
	}

	runErr := d.cmd.Run()

	// Drain any terminal responses (e.g. vim's DA2 capability queries) that
	// arrived on stdin while the editor owned the terminal. Without this,
	// those bytes leak into Bubble Tea's input reader or the parent shell.
	//
	// We read from the underlying *os.File directly so we can set a deadline.
	// The drain window is intentionally short — 50 ms is enough for any
	// in-flight terminal response to arrive.
	if f, ok := d.stdin.(*os.File); ok {
		_ = f.SetReadDeadline(time.Now().Add(50 * time.Millisecond))
		buf := make([]byte, 4096)
		for {
			_, err := f.Read(buf)
			if err != nil {
				break // deadline exceeded or EOF — done draining
			}
		}
		_ = f.SetReadDeadline(time.Time{}) // clear deadline
	}

	return runErr
}

// startEditorCmd suspends the Bubble Tea TUI, hands the terminal to the
// configured editor, then resumes and emits editorDoneMsg with the result.
//
// It uses tea.Exec with a custom drainExecCommand so that terminal capability
// query responses fired by the editor (e.g. vim's DA2 / t_RV) are drained
// from stdin before Bubble Tea's input loop resumes, preventing them from
// appearing as garbage input in the TUI or the parent shell.
func startEditorCmd(cfg config.Config, initialJSON []byte) tea.Cmd {
	// 1. Validate the editor binary before touching the filesystem.
	editorBin, err := validateEditorBin(cfg.Editor)
	if err != nil {
		return func() tea.Msg { return editorDoneMsg{Err: err} }
	}

	// 2. Create the temp dir and write the initial content synchronously.
	//    Cleanup happens inside the Exec callback after reading the result.
	dir, err := os.MkdirTemp("", "cumulus-*")
	if err != nil {
		return func() tea.Msg {
			return editorDoneMsg{Err: fmt.Errorf("creating temp dir: %w", err)}
		}
	}

	tmpFile := filepath.Join(dir, "item.json")
	if err := os.WriteFile(tmpFile, initialJSON, 0600); err != nil { // #nosec G306
		os.RemoveAll(dir)
		return func() tea.Msg {
			return editorDoneMsg{Err: fmt.Errorf("writing temp file: %w", err)}
		}
	}

	// 3. Use tea.Exec with our draining wrapper instead of tea.ExecProcess so
	//    we can flush stray terminal responses after the editor exits.
	//
	// #nosec G204 — not a shell-injection risk:
	//   • editorBin is a bare binary name (e.g. "nvim") validated against the
	//     hard-coded AllowedEditors allowlist; no user-supplied path or shell
	//     metacharacter can reach this argument.
	//   • tmpFile was constructed by us via os.MkdirTemp; it is never derived
	//     from user input.
	//   • exec.Command does not invoke a shell — arguments are passed directly
	//     to execve(2), so there is no command-injection surface.
	execCmd := &drainExecCommand{
		cmd: exec.Command(editorBin, tmpFile),
	}
	return tea.Exec(execCmd, func(execErr error) tea.Msg {
		defer os.RemoveAll(dir) // #nosec G defer — always clean up

		if execErr != nil {
			if errors.Is(execErr, exec.ErrNotFound) {
				names := make([]string, 0, len(config.AllowedEditors))
				for k := range config.AllowedEditors {
					names = append(names, k)
				}
				sort.Strings(names)
				return editorDoneMsg{Err: fmt.Errorf(
					"editor %q not found in PATH — install it or set a different editor in ~/.config/cumulus/config.toml\n(allowed: %s)",
					editorBin, strings.Join(names, ", "),
				)}
			}
			return editorDoneMsg{Err: fmt.Errorf("editor exited: %w", execErr)}
		}

		// Read the file back and validate it is still valid JSON.
		data, err := os.ReadFile(tmpFile) // #nosec G304 — path is our own temp file
		if err != nil {
			return editorDoneMsg{Err: fmt.Errorf("reading temp file after edit: %w", err)}
		}

		var v interface{}
		if err := json.Unmarshal(data, &v); err != nil {
			return editorDoneMsg{Err: fmt.Errorf("result is not valid JSON: %w", err)}
		}

		return editorDoneMsg{Data: data}
	})
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
