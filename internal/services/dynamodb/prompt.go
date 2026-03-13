package dynamodb

import (
	"strings"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
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

// Prompt is a single-line text input overlay.
type Prompt struct {
	kind   PromptKind
	input  textinput.Model
	label  string
	hint   string
	width  int
	height int
}

// NewFilterPrompt builds a filter-expression prompt.
func NewFilterPrompt() Prompt {
	return newPrompt(PromptFilter,
		"Filter expression",
		"e.g.  begins_with(#name, :val)  — press Enter to apply, Esc to cancel",
	)
}

// NewQueryPKPrompt builds a partition-key prompt.
func NewQueryPKPrompt(pkName string) Prompt {
	return newPrompt(PromptQueryPK,
		"Partition key  ("+pkName+")",
		"Enter the exact value — press Enter to query, Esc to cancel",
	)
}

// NewQuerySKPrompt builds an optional sort-key prompt.
func NewQuerySKPrompt(skName string) Prompt {
	return newPrompt(PromptQuerySK,
		"Sort key  ("+skName+")  — optional",
		"Leave empty to match all sort keys — press Enter to query, Esc to cancel",
	)
}

// NewConfirmPrompt builds a yes/no confirmation prompt.
func NewConfirmPrompt(message string) Prompt {
	p := newPrompt(PromptConfirm, message, "type 'yes' to confirm, Esc to cancel")
	p.input.Placeholder = "yes"
	return p
}

func newPrompt(kind PromptKind, label, hint string) Prompt {
	ti := textinput.New()
	ti.Focus()
	ti.CharLimit = 256
	ti.Width = 60
	ti.Prompt = "  › "
	ti.PromptStyle = lipgloss.NewStyle().Foreground(ui.ColorAccent)
	ti.TextStyle = lipgloss.NewStyle().Foreground(ui.ColorText)
	ti.PlaceholderStyle = lipgloss.NewStyle().Foreground(ui.ColorMuted)

	return Prompt{kind: kind, input: ti, label: label, hint: hint}
}

// Update handles keyboard input for the prompt. Returns (updated, cmd, done).
func (p Prompt) Update(msg tea.KeyMsg) (Prompt, tea.Cmd, bool) {
	switch msg.String() {
	case "enter":
		return p, func() tea.Msg {
			return promptDoneMsg{Value: strings.TrimSpace(p.input.Value())}
		}, true
	case "esc":
		return p, func() tea.Msg {
			return promptDoneMsg{Cancelled: true}
		}, true
	}
	updated, cmd := p.input.Update(msg)
	p.input = updated
	return p, cmd, false
}

// View renders the prompt as a centred overlay.
func (p Prompt) View(width, height int) string {
	label := ui.StyleTitle.Render(p.label)
	hint := ui.StyleDimmed.Render(p.hint)
	sep := lipgloss.NewStyle().Foreground(ui.ColorBorder).Render(strings.Repeat("─", 64))

	box := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(ui.ColorPrimary).
		Background(ui.ColorSurface).
		Padding(1, 3).
		Width(68).
		Render(label + "\n" + hint + "\n" + sep + "\n" + p.input.View())

	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ui.ColorBg),
	)
}
