package dynamodb

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	ddbtypes "github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/sumsar01/cumulus/internal/ui"
)

// DetailModel renders the full JSON of a single DynamoDB item in a scrollable
// viewport.
type DetailModel struct {
	viewport  viewport.Model
	item      map[string]ddbtypes.AttributeValue
	tableName string
	ready     bool
	width     int
	height    int
}

// NewDetailModel constructs the detail view for the given raw item.
// width and height should be the current terminal dimensions so the viewport
// can be initialised immediately without waiting for a WindowSizeMsg.
func NewDetailModel(item map[string]ddbtypes.AttributeValue, tableName string, width, height int) DetailModel {
	m := DetailModel{
		item:      item,
		tableName: tableName,
		width:     width,
		height:    height,
	}
	if width > 0 && height > 0 {
		m.initViewport(width, height)
	}
	return m
}

// initViewport creates or recreates the viewport for the given dimensions.
func (m *DetailModel) initViewport(width, height int) {
	// Layout: title(1) + sep(1) + viewport(N) + hints(1)
	// Each section is separated by a "\n" in View(), adding 3 more lines.
	// Total non-viewport lines = 1+1+1 = 3; each \n separator = 3 more → but
	// those newlines are *within* the returned string so the terminal counts
	// title+sep+hints = 3 logical lines consumed outside the viewport.
	headerH := 2 // title + sep
	footerH := 1 // hints
	vp := viewport.New(width, height-headerH-footerH)
	vp.Style = ui.StyleViewportBorder
	vp.SetContent(m.renderJSON())
	m.viewport = vp
	m.ready = true
}

func (m DetailModel) Init() tea.Cmd { return nil }

func (m DetailModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.initViewport(msg.Width, msg.Height)
		return m, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "y":
			// Copy JSON to clipboard via OSC 52 escape sequence — no exec.Command needed.
			// This works in most modern terminals and avoids shelling out.
			jsonStr := m.renderJSON()
			return m, copyToClipboard(jsonStr)
		}
	}

	vp, cmd := m.viewport.Update(msg)
	m.viewport = vp
	return m, cmd
}

func (m DetailModel) View() string {
	if !m.ready {
		content := ui.StyleMuted.Background(ui.ColorBg).Render("  Initialising…")
		return lipgloss.Place(m.width, m.height,
			lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	title := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width).Render(
		ui.StyleTitle.Background(ui.ColorBg).Render(m.tableName) +
			ui.StyleMuted.Background(ui.ColorBg).Render("  — item detail"),
	)

	scrollPct := fmt.Sprintf("  %3.f%%", m.viewport.ScrollPercent()*100)
	hints := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width).Render(
		ui.StyleMuted.Background(ui.ColorBg).Render("↑/↓ scroll  y copy JSON  esc back") +
			ui.StyleMuted.Background(ui.ColorBg).Render(scrollPct),
	)

	sep := ui.HorizontalSep(m.width)

	// Re-style every viewport line to force the theme background on blank
	// padding rows — bubbles/viewport fills unused height with a bare
	// lipgloss.NewStyle() (no background), which renders as terminal-default black.
	lineStyle := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width)
	vpLines := strings.Split(m.viewport.View(), "\n")
	for i, line := range vpLines {
		vpLines[i] = lineStyle.Render(line)
	}

	return title + "\n" + sep + "\n" + strings.Join(vpLines, "\n") + "\n" + hints
}

// renderJSON returns a pretty-printed JSON string of the item, styled with
// the app background colour so the viewport interior matches the theme.
func (m *DetailModel) renderJSON() string {
	// Convert DynamoDB AttributeValues to a generic map for JSON marshalling.
	generic := make(map[string]interface{}, len(m.item))
	for k, v := range m.item {
		generic[k] = attrValueToGeneric(v)
	}

	b, err := json.MarshalIndent(generic, "", "  ")
	if err != nil {
		return lipgloss.NewStyle().Background(ui.ColorBg).Foreground(ui.ColorDanger).
			Render(fmt.Sprintf("error rendering JSON: %v", err))
	}
	return lipgloss.NewStyle().Background(ui.ColorBg).Foreground(ui.ColorText).
		Render(string(b))
}

// copyToClipboard writes content to the terminal clipboard via OSC 52.
// OSC 52 is supported by most modern terminals (iTerm2, kitty, foot, WezTerm,
// tmux with set-clipboard on, etc.).  We write directly to /dev/tty so that
// the escape sequence reaches the terminal even while bubbletea holds stdout.
func copyToClipboard(content string) tea.Cmd {
	return func() tea.Msg {
		encoded := base64.StdEncoding.EncodeToString([]byte(content))
		seq := "\x1b]52;c;" + encoded + "\x07"

		tty, err := os.OpenFile("/dev/tty", os.O_WRONLY, 0) //nolint:gosec
		if err != nil {
			return ui.SetErrorMsg{Err: "clipboard: could not open /dev/tty: " + err.Error()}
		}
		defer tty.Close()

		if _, err := tty.WriteString(seq); err != nil {
			return ui.SetErrorMsg{Err: "clipboard: write failed: " + err.Error()}
		}

		return ui.SetStatusMsg{Msg: "copied to clipboard"}
	}
}
