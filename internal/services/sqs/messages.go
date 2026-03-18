package sqs

import (
	"fmt"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	sqstypes "github.com/aws/aws-sdk-go-v2/service/sqs/types"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/table"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/ui"
)

// msgPurpose distinguishes what an active prompt is for.
type msgPurpose int

const (
	purposeNone       msgPurpose = iota
	purposeDelete                // confirm delete of a single message
	purposeRedriveArn            // confirm source ARN before starting redrive
)

// MessagesModel displays messages polled from an SQS queue.
type MessagesModel struct {
	cfg      aws.Config
	queueURL string

	messages []sqstypes.Message // currently displayed batch
	rawRows  []table.Row        // rows without cursor prefix

	// redrive state — populated after getQueueAttributesCmd returns
	dlqArn         string
	sourceQueueArn string

	// prompt overlay
	activePrompt  *Prompt
	promptPurpose msgPurpose

	table   table.Model
	spinner spinner.Model
	loading bool
	err     error

	width      int
	height     int
	fullHeight int // full terminal height before the status bar deduction
}

// NewMessagesModel constructs the messages view for the given queue.
func NewMessagesModel(cfg aws.Config, queueURL string) MessagesModel {
	t := table.New(
		table.WithFocused(true),
		table.WithStyles(ui.DynamoTableStyles),
	)
	return MessagesModel{
		cfg:      cfg,
		queueURL: queueURL,
		table:    t,
		spinner:  ui.NewSpinner(),
	}
}

// Init satisfies tea.Model; no automatic fetch — user presses 'r'.
func (m MessagesModel) Init() tea.Cmd { return nil }

// Update handles all messages for the messages view.
func (m MessagesModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	// Prompt overlay takes priority over everything except non-key messages.
	if m.activePrompt != nil {
		if km, ok := msg.(tea.KeyMsg); ok {
			p, cmd, done := m.activePrompt.Update(km)
			if done {
				m.activePrompt = nil
				return m, cmd // emits promptDoneMsg
			}
			m.activePrompt = &p
			return m, cmd
		}
	}

	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.fullHeight = msg.Height + 2 // restore full height (app strips 2 for status bar)
		m.resizeTable()

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.messages = nil
		m.rawRows = nil
		m.dlqArn = ""
		m.sourceQueueArn = ""
		m.err = nil
		m.table.SetRows(nil)
		m.table.SetColumns(nil)
		return m, nil

	case spinner.TickMsg:
		if m.loading {
			sp, cmd := m.spinner.Update(msg)
			m.spinner = sp
			return m, cmd
		}

	case messagesReceivedMsg:
		m.loading = false
		m.messages = msg.messages
		m.rebuildTable()
		return m, nil

	case messageDeletedMsg:
		// Re-poll after deletion so the list is fresh.
		m.loading = true
		m.messages = nil
		m.rawRows = nil
		m.table.SetRows(nil)
		return m, tea.Batch(m.spinner.Tick, receiveMessagesCmd(m.cfg, m.queueURL))

	case redriveInfoMsg:
		m.loading = false
		m.dlqArn = msg.dlqArn
		m.sourceQueueArn = msg.sourceQueueArn
		// Show the confirm prompt with the detected ARN.
		dest := msg.sourceQueueArn
		if dest == "" {
			dest = "(no source queue detected — will use default)"
		}
		p := NewConfirmPrompt(fmt.Sprintf("Redrive DLQ back to source?\n\n%s", dest))
		m.activePrompt = &p
		m.promptPurpose = purposeRedriveArn
		return m, nil

	case redriveStartedMsg:
		return m, func() tea.Msg {
			return ui.SetStatusMsg{Msg: "Redrive task started"}
		}

	case promptDoneMsg:
		if msg.Cancelled {
			m.activePrompt = nil
			m.promptPurpose = purposeNone
			return m, nil
		}
		return m.handlePromptDone()

	case awspkg.ErrMsg:
		m.loading = false
		m.err = msg.Err
		return m, nil

	case tea.KeyMsg:
		return m.handleKey(msg)
	}

	t, cmd := m.table.Update(msg)
	m.table = t
	m.injectCursor()
	return m, cmd
}

func (m MessagesModel) handleKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "r":
		// Poll for messages.
		m.loading = true
		m.err = nil
		m.messages = nil
		m.rawRows = nil
		m.table.SetRows(nil)
		return m, tea.Batch(m.spinner.Tick, receiveMessagesCmd(m.cfg, m.queueURL))

	case "d":
		// Delete the selected message — show confirm prompt first.
		msg := m.selectedMessage()
		if msg == nil {
			return m, nil
		}
		p := NewConfirmPrompt(fmt.Sprintf("Delete message?\n\n%s", truncate(aws.ToString(msg.MessageId), 36)))
		m.activePrompt = &p
		m.promptPurpose = purposeDelete
		return m, nil

	case "R":
		// Start DLQ redrive — fetch queue attributes to detect the source ARN.
		m.loading = true
		m.err = nil
		return m, tea.Batch(m.spinner.Tick, getQueueAttributesCmd(m.cfg, m.queueURL))

	case "enter":
		selected := m.selectedMessage()
		if selected == nil {
			return m, nil
		}
		detail := NewMessageDetailModel(*selected, queueDisplayName(m.queueURL), m.width, m.height)
		return m, tea.Batch(
			detail.Init(),
			func() tea.Msg { return ui.PushMsg{Model: detail} },
		)
	}

	t, cmd := m.table.Update(msg)
	m.table = t
	m.injectCursor()
	return m, cmd
}

func (m *MessagesModel) handlePromptDone() (tea.Model, tea.Cmd) {
	switch m.promptPurpose {
	case purposeDelete:
		selected := m.selectedMessage()
		if selected == nil || selected.ReceiptHandle == nil {
			return m, nil
		}
		return m, deleteMessageCmd(m.cfg, m.queueURL, *selected.ReceiptHandle)

	case purposeRedriveArn:
		// User confirmed — start the move task.
		return m, redriveCmd(m.cfg, m.dlqArn, m.sourceQueueArn)
	}
	return m, nil
}

// IsTextInputActive implements ui.TextInputActive.
func (m MessagesModel) IsTextInputActive() bool { return m.activePrompt != nil }

// View renders the messages table.
func (m MessagesModel) View() string {
	if m.loading {
		content := ui.StyleMuted.Background(ui.ColorBg).Render(
			m.spinner.View() + "  Loading…",
		)
		return lipgloss.Place(m.width, m.fullHeight,
			lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}
	if m.err != nil {
		content := ui.StyleDanger.Background(ui.ColorBg).Render("  "+m.err.Error()) +
			"\n" + ui.StyleMuted.Background(ui.ColorBg).Render("  r  retry")
		return lipgloss.Place(m.width, m.fullHeight,
			lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	if m.activePrompt != nil {
		return m.activePrompt.View(m.width, m.fullHeight)
	}

	if len(m.messages) == 0 {
		hint := ui.StyleMuted.Background(ui.ColorBg).Render(
			"No messages loaded.\n\n" +
				ui.StyleKey.Background(ui.ColorBg).Render("r") +
				ui.StyleMuted.Background(ui.ColorBg).Render("  poll for messages"),
		)
		return lipgloss.Place(m.width, m.fullHeight,
			lipgloss.Center, lipgloss.Center, hint,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	header := m.headerView()
	sep := ui.HorizontalSep(m.width)
	hints := ui.RenderHints(m.width, [][2]string{
		{"enter", "detail"}, {"r", "poll"}, {"d", "delete"}, {"R", "redrive DLQ"},
	})

	// Re-style table lines to force the theme background colour (same technique
	// as DynamoDB ItemsModel to avoid terminal-default black on blank rows).
	lineStyle := lipgloss.NewStyle().Background(ui.ColorBg).MaxWidth(m.width).Width(m.width)
	selectedStyle := lipgloss.NewStyle().
		Background(ui.ColorHighlight).
		Foreground(ui.ColorText).
		Bold(true).
		MaxWidth(m.width).
		Width(m.width)
	selectedLine := 2 + m.table.Cursor()
	tableLines := strings.Split(m.table.View(), "\n")
	for i, line := range tableLines {
		if i == selectedLine {
			clean := ansi.Strip(line)
			rendered := selectedStyle.Render(clean)
			rendered = strings.Replace(rendered, "› ", ui.StyleCursor.Render("›")+" ", 1)
			tableLines[i] = rendered
		} else {
			tableLines[i] = lineStyle.Render(line)
		}
	}

	return header + "\n" + sep + "\n" + strings.Join(tableLines, "\n") + "\n" + hints
}

func (m MessagesModel) headerView() string {
	title := ui.StyleTitle.Background(ui.ColorBg).Render(queueDisplayName(m.queueURL))
	count := ui.StyleCount.Render(fmt.Sprintf("  %d messages", len(m.messages)))
	return ui.StyleItemsHeader.Width(m.width).Render(title + count)
}

// ── helpers ───────────────────────────────────────────────────────────────────

func (m *MessagesModel) selectedMessage() *sqstypes.Message {
	idx := m.table.Cursor()
	if idx < 0 || idx >= len(m.messages) {
		return nil
	}
	return &m.messages[idx]
}

func (m *MessagesModel) rebuildTable() {
	if len(m.messages) == 0 {
		m.table.SetRows(nil)
		m.table.SetColumns(nil)
		return
	}

	cols := []table.Column{
		{Title: "ID", Width: 10},
		{Title: "Body", Width: 10},
		{Title: "Sent", Width: 20},
		{Title: "Receives", Width: 8},
	}

	// Distribute width evenly: 4 columns, each gets padding of 2 from bubbles.
	if m.width > 0 {
		available := m.width - 2 - len(cols)*2
		base := available / len(cols)
		rem := available % len(cols)
		if base < 1 {
			base = 1
			rem = 0
		}
		for i := range cols {
			w := base
			if i < rem {
				w++
			}
			if i == 0 {
				w += 2 // cursor prefix
			}
			cols[i].Width = w
		}
	}

	rawRows := make([]table.Row, len(m.messages))
	for i, msg := range m.messages {
		id := truncate(aws.ToString(msg.MessageId), cols[0].Width-2)
		body := truncate(aws.ToString(msg.Body), cols[1].Width-2)
		sent := formatTimestamp(msg.Attributes["SentTimestamp"])
		receives := msg.Attributes["ApproximateReceiveCount"]
		rawRows[i] = table.Row{id, body, sent, receives}
	}
	m.rawRows = rawRows

	m.table.SetColumns(cols)
	m.injectCursor()
	m.resizeTable()
}

func (m *MessagesModel) resizeTable() {
	if m.height > 4 {
		m.table.SetHeight(m.height - 4)
	}
	if m.width > 0 {
		m.table.SetWidth(m.width)
		m.table.SetStyles(ui.DynamoTableStyles)
	}
}

func (m *MessagesModel) injectCursor() {
	if len(m.rawRows) == 0 {
		m.table.SetRows(nil)
		return
	}
	cursor := m.table.Cursor()
	display := make([]table.Row, len(m.rawRows))
	for i, raw := range m.rawRows {
		row := make(table.Row, len(raw))
		copy(row, raw)
		if len(row) > 0 {
			if i == cursor {
				row[0] = "› " + raw[0]
			} else {
				row[0] = "  " + raw[0]
			}
		}
		display[i] = row
	}
	m.table.SetRows(display)
}

// truncate shortens s to maxLen runes, appending "…" if truncated.
func truncate(s string, maxLen int) string {
	runes := []rune(s)
	if len(runes) <= maxLen {
		return s
	}
	if maxLen <= 1 {
		return "…"
	}
	return string(runes[:maxLen-1]) + "…"
}

// formatTimestamp converts an SQS epoch-milliseconds string to a short
// human-readable format.  Returns the raw value if parsing fails.
func formatTimestamp(ms string) string {
	if ms == "" {
		return "—"
	}
	var epoch int64
	if _, err := fmt.Sscan(ms, &epoch); err != nil {
		return ms
	}
	t := time.UnixMilli(epoch).UTC()
	return t.Format("2006-01-02 15:04:05")
}
