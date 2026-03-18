// Package ui contains the root Bubble Tea application model for cumulus.
// It owns the view stack and routes global key bindings before delegating to
// the top-most view.
package ui

import (
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/config"

	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
)

// PushMsg asks the app to push a new model onto the view stack.
type PushMsg struct{ Model tea.Model }

// TextInputActive is an optional interface that views can implement to signal
// that the user is currently typing into a text field. When active, the app
// routes all key events directly to the view, bypassing global shortcuts
// (except ctrl+c) so that keys like q, p, ? can be typed freely.
type TextInputActive interface {
	IsTextInputActive() bool
}

// PopMsg asks the app to pop the top-most model from the view stack.
type PopMsg struct{}

// SetBreadcrumbMsg updates the status bar breadcrumb from within a child view.
type SetBreadcrumbMsg struct{ Crumbs []string }

// SetErrorMsg displays a transient error in the status bar.
type SetErrorMsg struct{ Err string }

// SetStatusMsg displays a transient informational message in the status bar
// (rendered in success/green color, not the error color).
type SetStatusMsg struct{ Msg string }

// App is the root Bubble Tea model. It manages a stack of sub-models and a
// persistent status bar. Only the top of the stack receives Update/View calls;
// the status bar is always rendered.
type App struct {
	stack  []tea.Model
	status StatusBar
	cfg    aws.Config
	appCfg config.Config
	width  int
	height int

	// overlays rendered on top of everything
	profilePicker *ProfilePicker
	themePicker   *ThemePicker
	helpOverlay   *HelpOverlay
}

// New creates the root App model. navigator is the initial view pushed onto
// the stack (typically the service navigator).
func New(navigator tea.Model, cfg aws.Config, appCfg config.Config, profile, region string) App {
	return App{
		stack:  []tea.Model{navigator},
		cfg:    cfg,
		appCfg: appCfg,
		status: StatusBar{
			Profile: profile,
			Region:  region,
		},
	}
}

func (a App) Init() tea.Cmd {
	if len(a.stack) == 0 {
		return nil
	}
	return a.stack[0].Init()
}

func (a App) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmds []tea.Cmd

	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		a.width = msg.Width
		a.height = msg.Height
		a.status.Width = msg.Width
		// Propagate to active view (minus status bar height).
		// Reserve 2 lines: 1 for the main bar + 1 for the optional error line.
		if len(a.stack) > 0 {
			inner := tea.WindowSizeMsg{Width: msg.Width, Height: msg.Height - 2}
			updated, cmd := a.top().Update(inner)
			a.setTop(updated)
			cmds = append(cmds, cmd)
		}
		return a, tea.Batch(cmds...)

	case awspkg.ProfileChangedMsg:
		a.cfg = msg.Cfg
		a.status.Profile = msg.Profile
		a.status.Region = msg.Region
		a.status.Err = ""
		a.status.Status = ""
		// Persist the chosen profile so it is restored on next launch.
		appCfg := a.appCfg
		appCfg.LastProfile = msg.Profile
		a.appCfg = appCfg
		cmds = append(cmds, func() tea.Msg {
			_ = config.Save(appCfg) // best-effort; ignore errors
			return nil
		})
		// Broadcast to the entire stack.
		for i, m := range a.stack {
			updated, cmd := m.Update(msg)
			a.stack[i] = updated
			cmds = append(cmds, cmd)
		}
		return a, tea.Batch(cmds...)

	case ThemeChangedMsg:
		// ApplyTheme was already called live by the picker; here we persist.
		appCfg := a.appCfg
		appCfg.Theme = msg.Name
		a.appCfg = appCfg
		cmds = append(cmds, func() tea.Msg {
			_ = config.Save(appCfg) // best-effort; ignore errors
			return nil
		})
		return a, tea.Batch(cmds...)

	case awspkg.ErrMsg:
		a.status.Err = msg.Err.Error()
		return a, nil

	case SetErrorMsg:
		a.status.Err = msg.Err
		a.status.Status = ""
		return a, nil

	case SetStatusMsg:
		a.status.Status = msg.Msg
		a.status.Err = ""
		return a, nil

	case SetBreadcrumbMsg:
		a.status.Breadcrumb = msg.Crumbs
		return a, nil

	case PushMsg:
		a.stack = append(a.stack, msg.Model)
		// Immediately size the new model to the current terminal dimensions.
		inner := tea.WindowSizeMsg{Width: a.width, Height: a.height - 2}
		updated, sizeCmd := a.top().Update(inner)
		a.setTop(updated)
		return a, tea.Batch(updated.Init(), sizeCmd)

	case PopMsg:
		if len(a.stack) > 1 {
			a.stack = a.stack[:len(a.stack)-1]
		}
		return a, nil

	case tea.KeyMsg:
		return a.handleKey(msg)
	}

	// Route non-key messages to the profile picker when it is open.
	if a.profilePicker != nil {
		picker, cmd := a.profilePicker.HandleMsg(msg)
		a.profilePicker = picker
		cmds = append(cmds, cmd)
		return a, tea.Batch(cmds...)
	}

	// Delegate to the top of the stack.
	if len(a.stack) > 0 {
		updated, cmd := a.top().Update(msg)
		a.setTop(updated)
		cmds = append(cmds, cmd)
	}

	return a, tea.Batch(cmds...)
}

func (a App) View() string {
	body := ""
	if len(a.stack) > 0 {
		body = a.top().View()
	}

	// Overlay rendering.
	if a.helpOverlay != nil {
		body = a.helpOverlay.View(a.width, a.height-1)
	} else if a.profilePicker != nil {
		body = a.profilePicker.View(a.width, a.height-1)
	} else if a.themePicker != nil {
		body = a.themePicker.View(a.width, a.height-1)
	}

	return body + "\n" + a.status.View()
}

func (a *App) top() tea.Model {
	return a.stack[len(a.stack)-1]
}

func (a *App) setTop(m tea.Model) {
	a.stack[len(a.stack)-1] = m
}

func (a App) handleKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	// Handle overlays first.
	if a.helpOverlay != nil {
		if msg.String() == "?" || msg.String() == "esc" {
			a.helpOverlay = nil
		}
		return a, nil
	}
	if a.profilePicker != nil {
		picker, cmd, done := a.profilePicker.Update(msg)
		if done {
			a.profilePicker = nil
			return a, cmd // cmd is either LoadProfileCmd or nil (cancel)
		}
		a.profilePicker = picker
		return a, cmd
	}
	if a.themePicker != nil {
		picker, cmd, done := a.themePicker.Update(msg)
		if done {
			a.themePicker = nil
			return a, cmd // cmd is either ThemeChangedMsg or nil (cancel)
		}
		a.themePicker = picker
		return a, cmd
	}

	// If the active view has a text input open, bypass all global
	// shortcuts — only ctrl+c remains a hard quit.
	if len(a.stack) > 0 {
		if tia, ok := a.top().(TextInputActive); ok && tia.IsTextInputActive() {
			if msg.String() == "ctrl+c" {
				return a, tea.Quit
			}
			updated, cmd := a.top().Update(msg)
			a.setTop(updated)
			return a, cmd
		}
	}

	// Global bindings.
	switch msg.String() {
	case "ctrl+c":
		return a, tea.Quit
	case "?":
		ho := NewHelpOverlay()
		a.helpOverlay = &ho
		return a, nil
	case "p":
		pp, cmd := NewProfilePicker(a.status.Profile)
		a.profilePicker = &pp
		return a, cmd
	case "t":
		tp := NewThemePicker()
		a.themePicker = &tp
		return a, nil
	case "esc":
		if len(a.stack) > 1 {
			a.stack = a.stack[:len(a.stack)-1]
			// Clear breadcrumb beyond navigator.
			if len(a.stack) == 1 {
				a.status.Breadcrumb = nil
			}
		}
		return a, nil
	}

	// Unhandled key: delegate to top of stack.
	if len(a.stack) > 0 {
		updated, cmd := a.top().Update(msg)
		a.setTop(updated)
		return a, cmd
	}
	return a, nil
}
