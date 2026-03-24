package sqs

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"strings"

	sqstypes "github.com/aws/aws-sdk-go-v2/service/sqs/types"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/sumsar01/cumulus/internal/ui"
)

// MessageDetailModel renders the full body of a single SQS message in a
// scrollable viewport.
type MessageDetailModel struct {
	viewport  viewport.Model
	message   sqstypes.Message
	queueName string
	ready     bool
	width     int
	height    int
}

// NewMessageDetailModel constructs the detail view for the given message.
// width and height should be the current terminal dimensions.
func NewMessageDetailModel(message sqstypes.Message, queueName string, width, height int) MessageDetailModel {
	m := MessageDetailModel{
		message:   message,
		queueName: queueName,
		width:     width,
		height:    height,
	}
	if width > 0 && height > 0 {
		m.initViewport(width, height)
	}
	return m
}

func (m *MessageDetailModel) initViewport(width, height int) {
	// Layout: title(1) + sep(1) + viewport(N) + hints(1)
	headerH := 2
	footerH := 1
	vp := viewport.New(width, height-headerH-footerH)
	vp.Style = ui.StyleViewportBorder
	vp.SetContent(m.renderBody())
	m.viewport = vp
	m.ready = true
}

// Init satisfies tea.Model.
func (m MessageDetailModel) Init() tea.Cmd { return nil }

// Update handles scroll, copy, and resize.
func (m MessageDetailModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.initViewport(msg.Width, msg.Height)
		return m, nil

	case tea.KeyMsg:
		switch msg.String() {
		case "y":
			// Copy message body to clipboard via OSC 52 — no exec.Command needed.
			body := ""
			if m.message.Body != nil {
				body = *m.message.Body
			}
			return m, sqsCopyToClipboard(body)
		}
	}

	vp, cmd := m.viewport.Update(msg)
	m.viewport = vp
	return m, cmd
}

// View renders the detail screen.
func (m MessageDetailModel) View() string {
	if !m.ready {
		content := ui.StyleMuted.Background(ui.ColorBg).Render("  Initialising…")
		return lipgloss.Place(m.width, m.height,
			lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	msgID := ""
	if m.message.MessageId != nil {
		msgID = *m.message.MessageId
	}
	title := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width).Render(
		ui.StyleTitle.Background(ui.ColorBg).Render(m.queueName) +
			ui.StyleMuted.Background(ui.ColorBg).Render("  — "+truncate(msgID, 36)),
	)

	scrollPct := fmt.Sprintf("  %3.f%%", m.viewport.ScrollPercent()*100)
	hints := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width).Render(
		ui.StyleMuted.Background(ui.ColorBg).Render("↑/↓ scroll  y copy body  esc back") +
			ui.StyleMuted.Background(ui.ColorBg).Render(scrollPct),
	)

	sep := ui.HorizontalSep(m.width)

	// Force theme background on every viewport line (same technique as
	// DynamoDB DetailModel — without this, blank rows render as terminal-default black).
	lineStyle := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width)
	vpLines := strings.Split(m.viewport.View(), "\n")
	for i, line := range vpLines {
		vpLines[i] = lineStyle.Render(line)
	}

	return title + "\n" + sep + "\n" + strings.Join(vpLines, "\n") + "\n" + hints
}

// renderBody pretty-prints the message body as JSON if possible, otherwise
// returns it as raw text, styled with the app background colour.
func (m *MessageDetailModel) renderBody() string {
	body := ""
	if m.message.Body != nil {
		body = *m.message.Body
	}

	style := lipgloss.NewStyle().Background(ui.ColorBg).Foreground(ui.ColorText)
	var generic interface{}
	if json.Unmarshal([]byte(body), &generic) == nil {
		if b, err := json.MarshalIndent(generic, "", "  "); err == nil {
			return style.Render(string(b))
		}
	}
	return style.Render(body)
}

// sqsCopyToClipboard writes content to the terminal clipboard via OSC 52.
// This works in most modern terminals (iTerm2, kitty, foot, WezTerm, tmux with
// set-clipboard on) and avoids shelling out to an external command.
func sqsCopyToClipboard(content string) tea.Cmd {
	return func() tea.Msg {
		encoded := base64.StdEncoding.EncodeToString([]byte(content))
		seq := "\x1b]52;c;" + encoded + "\x07"

		// #nosec G304 / nolint:gosec — opening /dev/tty is intentional.
		// The path is a hard-coded constant, not user input.
		// The permission argument (0) is irrelevant: O_WRONLY on an existing
		// character device never creates a new file, so the kernel ignores the
		// mode bits entirely.
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
