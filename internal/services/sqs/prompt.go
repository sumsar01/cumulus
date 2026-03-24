package sqs

import (
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/huh"
	"github.com/charmbracelet/lipgloss"
	"github.com/sumsar01/cumulus/internal/ui"
)

// promptDoneMsg is emitted when the user confirms or cancels a prompt.
type promptDoneMsg struct {
	Value     string
	Cancelled bool
}

// PromptKind distinguishes what we are prompting for.
type PromptKind int

const (
	PromptConfirm PromptKind = iota // yes/no confirmation
)

// Prompt is a single-field huh overlay that collects one piece of input.
type Prompt struct {
	kind  PromptKind
	field huh.Field
	value string
}

// NewConfirmPrompt builds a yes/no confirmation prompt.
func NewConfirmPrompt(message string) Prompt {
	p := Prompt{kind: PromptConfirm}
	p.field = huh.NewConfirm().
		Title(message).
		Description("Select Yes to confirm, No or Esc to cancel").
		Affirmative("Yes").
		Negative("No").
		WithTheme(ui.CumulusHuhTheme())
	_ = p.field.Focus()
	return p
}

// Update handles all messages for the prompt. Returns (updated, cmd, done).
// done is true when the user has confirmed or cancelled.
func (p Prompt) Update(msg tea.Msg) (Prompt, tea.Cmd, bool) {
	// Esc always cancels regardless of field type.
	if km, ok := msg.(tea.KeyMsg); ok && km.String() == "esc" {
		return p, func() tea.Msg { return promptDoneMsg{Cancelled: true} }, true
	}

	updated, cmd := p.field.Update(msg)
	p.field = updated.(huh.Field)

	if err := p.field.Error(); err == nil {
		if km, ok := msg.(tea.KeyMsg); ok && km.String() == "enter" {
			return p, p.doneCmd(), true
		}
	}

	return p, cmd, false
}

// doneCmd emits a promptDoneMsg with the current value.
func (p Prompt) doneCmd() tea.Cmd {
	return func() tea.Msg {
		if p.field.GetValue().(bool) {
			return promptDoneMsg{Value: "yes"}
		}
		return promptDoneMsg{Cancelled: true}
	}
}

// View renders the prompt as a centred overlay.
func (p Prompt) View(width, height int) string {
	fieldView := p.field.View()
	inner := ui.StylePromptInner.Width(sqsMin(68, width-8)).Render(fieldView)
	box := ui.StylePromptBox.Render(inner)
	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ui.ColorBg),
	)
}

// sqsMin is a helper to avoid importing math for a single comparison.
func sqsMin(a, b int) int {
	if a < b {
		return a
	}
	return b
}
