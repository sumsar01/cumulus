package dynamodb

import (
	"strings"

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
	PromptFilter  PromptKind = iota // FilterExpression
	PromptQueryPK                   // Partition key value
	PromptQuerySK                   // Sort key value (optional)
	PromptConfirm                   // yes/no confirmation (delete)
)

// Prompt is a single-field huh overlay that collects one piece of input.
type Prompt struct {
	kind  PromptKind
	field huh.Field
	value string // bound string for Input fields
}

// NewFilterPrompt builds a filter-expression prompt.
func NewFilterPrompt() Prompt {
	return newInputPrompt(PromptFilter,
		"Filter expression",
		"e.g.  begins_with(#name, :val)  — Enter to apply, Esc to cancel",
		"",
	)
}

// NewQueryPKPrompt builds a partition-key prompt.
func NewQueryPKPrompt(pkName string) Prompt {
	return newInputPrompt(PromptQueryPK,
		"Partition key  ("+pkName+")",
		"Enter the exact value — Enter to query, Esc to cancel",
		"",
	)
}

// NewQuerySKPrompt builds an optional sort-key prompt.
func NewQuerySKPrompt(skName string) Prompt {
	return newInputPrompt(PromptQuerySK,
		"Sort key  ("+skName+")  — optional",
		"Leave empty to match all sort keys — Enter to query, Esc to cancel",
		"",
	)
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
	// Focus the field immediately.
	_ = p.field.Focus()
	return p
}

func newInputPrompt(kind PromptKind, label, hint, placeholder string) Prompt {
	p := Prompt{kind: kind}
	p.field = huh.NewInput().
		Title(label).
		Description(hint).
		Placeholder(placeholder).
		CharLimit(256).
		Value(&p.value).
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

	// huh signals completion when the field is no longer focused after update
	// (it blurs itself on Enter). We detect this by checking field.Error() == nil
	// and whether the field's internal state indicates submission.
	// The reliable way: check if the field blurred itself (lost focus).
	if err := p.field.Error(); err == nil {
		// Check for Enter key — field will have processed the value.
		if km, ok := msg.(tea.KeyMsg); ok && km.String() == "enter" {
			return p, p.doneCmd(), true
		}
	}

	return p, cmd, false
}

// doneCmd emits a promptDoneMsg with the current value.
func (p Prompt) doneCmd() tea.Cmd {
	return func() tea.Msg {
		switch p.kind {
		case PromptConfirm:
			if p.field.GetValue().(bool) {
				return promptDoneMsg{Value: "yes"}
			}
			return promptDoneMsg{Cancelled: true}
		default:
			return promptDoneMsg{Value: strings.TrimSpace(p.value)}
		}
	}
}

// View renders the prompt as a centred overlay.
func (p Prompt) View(width, height int) string {
	fieldView := p.field.View()

	inner := ui.StylePromptInner.Width(min(68, width-8)).Render(fieldView)
	box := ui.StylePromptBox.Render(inner)

	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ui.ColorBg),
	)
}

func min(a, b int) int {
	if a < b {
		return a
	}
	return b
}
