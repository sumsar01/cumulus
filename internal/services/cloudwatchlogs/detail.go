package cloudwatchlogs

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/sumsar01/cumulus/internal/ui"
)

// EventDetailModel renders the full message of a single log event in a
// scrollable viewport.
type EventDetailModel struct {
	viewport   viewport.Model
	event      logEvent
	groupName  string
	streamName string
	ready      bool
	width      int
	height     int
}

// NewEventDetailModel constructs the detail view for the given log event.
// width and height should be the current terminal dimensions so the viewport
// can be initialised immediately without waiting for a WindowSizeMsg.
func NewEventDetailModel(event logEvent, groupName, streamName string, width, height int) EventDetailModel {
	m := EventDetailModel{
		event:      event,
		groupName:  groupName,
		streamName: streamName,
		width:      width,
		height:     height,
	}
	if width > 0 && height > 0 {
		m.initViewport(width, height)
	}
	return m
}

// initViewport creates or recreates the viewport for the given dimensions.
func (m *EventDetailModel) initViewport(width, height int) {
	// Layout: title(1) + sep(1) + viewport(N) + hints(1)
	headerH := 2
	footerH := 1
	vp := viewport.New(width, height-headerH-footerH)
	vp.Style = ui.StyleViewportBorder
	vp.SetContent(m.renderContent())
	m.viewport = vp
	m.ready = true
}

// Init is a no-op; content is set in the constructor.
func (m EventDetailModel) Init() tea.Cmd { return nil }

// Update handles messages for the event-detail view.
func (m EventDetailModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.initViewport(msg.Width, msg.Height)
		return m, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "y":
			// Copy the raw event message to the clipboard via OSC 52.
			return m, copyEventToClipboard(m.event.message)
		}
	}

	vp, cmd := m.viewport.Update(msg)
	m.viewport = vp
	return m, cmd
}

// View renders the event detail viewport.
func (m EventDetailModel) View() string {
	if !m.ready {
		content := ui.StyleMuted.Background(ui.ColorBg).Render("  Initialising…")
		return lipgloss.Place(m.width, m.height,
			lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	ts := ""
	if !m.event.timestamp.IsZero() {
		ts = "  " + ui.StyleMuted.Background(ui.ColorBg).Render(
			m.event.timestamp.Local().Format("2006-01-02 15:04:05"),
		)
	}
	title := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width).Render(
		ui.StyleTitle.Background(ui.ColorBg).Render(m.streamName) + ts,
	)

	scrollPct := fmt.Sprintf("  %3.f%%", m.viewport.ScrollPercent()*100)
	hints := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width).Render(
		ui.StyleMuted.Background(ui.ColorBg).Render("↑/↓ scroll  y copy  esc back") +
			ui.StyleMuted.Background(ui.ColorBg).Render(scrollPct),
	)

	sep := ui.HorizontalSep(m.width)

	// Force theme background on every viewport line.
	lineStyle := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width)
	vpLines := strings.Split(m.viewport.View(), "\n")
	for i, line := range vpLines {
		vpLines[i] = lineStyle.Render(line)
	}

	return title + "\n" + sep + "\n" + strings.Join(vpLines, "\n") + "\n" + hints
}

// renderContent returns a styled string for the viewport.
// If the message is valid JSON it is pretty-printed; otherwise it is shown as-is.
func (m *EventDetailModel) renderContent() string {
	msg := strings.TrimRight(m.event.message, "\n")

	// Attempt pretty-print if the message looks like JSON.
	if strings.HasPrefix(strings.TrimSpace(msg), "{") || strings.HasPrefix(strings.TrimSpace(msg), "[") {
		var v interface{}
		if err := json.Unmarshal([]byte(msg), &v); err == nil {
			if b, err := json.MarshalIndent(v, "", "  "); err == nil {
				msg = string(b)
			}
		}
	}

	return lipgloss.NewStyle().Background(ui.ColorBg).Foreground(ui.ColorText).Render(msg)
}

// copyEventToClipboard writes the event message to the terminal clipboard via OSC 52.
// OSC 52 is supported by most modern terminals (iTerm2, kitty, foot, WezTerm,
// tmux with set-clipboard on, etc.). We write directly to /dev/tty so that
// the escape sequence reaches the terminal even while bubbletea holds stdout.
func copyEventToClipboard(content string) tea.Cmd {
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
