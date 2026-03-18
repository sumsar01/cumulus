package dynamodb

import (
	"context"
	"fmt"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/config"
	"github.com/sumsar01/cumulus/internal/ui"
)

// tablesLoadedMsg carries the result of a ListTables API call.
type tablesLoadedMsg struct{ tables []string }

// TablesModel lists all DynamoDB tables and lets the user select one.
type TablesModel struct {
	cfg       aws.Config
	appCfg    config.Config
	tables    []string
	cursor    int
	filter    string
	filtering bool // true when the user is actively typing a filter
	spinner   spinner.Model
	loading   bool
	err       error
	width     int
	height    int
}

// NewTablesModel constructs the tables view.
func NewTablesModel(cfg aws.Config, appCfg config.Config) TablesModel {
	return TablesModel{cfg: cfg, appCfg: appCfg, spinner: ui.NewSpinner()}
}

func (m TablesModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, fetchTablesCmd(m.cfg))
}

func (m TablesModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.tables = nil
		m.cursor = 0
		m.filter = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchTablesCmd(m.cfg))

	case tablesLoadedMsg:
		m.loading = false
		m.tables = msg.tables
		m.cursor = 0
		return m, nil

	case awspkg.ErrMsg:
		m.loading = false
		m.err = msg.Err
		return m, nil

	case spinner.TickMsg:
		if m.loading {
			sp, cmd := m.spinner.Update(msg)
			m.spinner = sp
			return m, cmd
		}

	case tea.KeyMsg:
		// While filtering, all input goes to the filter field.
		if m.filtering {
			switch msg.String() {
			case "esc":
				m.filtering = false
				m.filter = ""
				m.cursor = 0
			case "enter":
				// Confirm filter, exit filter-input mode but keep the filter active.
				m.filtering = false
			case "backspace":
				if len(m.filter) > 0 {
					m.filter = m.filter[:len(m.filter)-1]
					m.cursor = 0
				}
			default:
				if len(msg.String()) == 1 && msg.String() != "/" {
					m.filter += msg.String()
					m.cursor = 0
				}
			}
			return m, nil
		}

		// Normal (non-filtering) keybindings.
		switch msg.String() {
		case "/":
			m.filtering = true
		case "r":
			m.loading = true
			m.err = nil
			m.tables = nil
			m.cursor = 0
			m.filter = ""
			m.filtering = false
			return m, tea.Batch(m.spinner.Tick, fetchTablesCmd(m.cfg))
		case "up", "k":
			if m.cursor > 0 {
				m.cursor--
			}
		case "down", "j":
			if m.cursor < len(m.visible())-1 {
				m.cursor++
			}
		case "esc":
			if m.filter != "" {
				m.filter = ""
				m.cursor = 0
				return m, nil
			}
		case "enter":
			vis := m.visible()
			if len(vis) == 0 {
				return m, nil
			}
			tableName := vis[m.cursor]
			itemsModel := NewItemsModel(m.cfg, m.appCfg, tableName)
			return m, tea.Batch(
				itemsModel.Init(),
				func() tea.Msg { return ui.PushMsg{Model: itemsModel} },
				func() tea.Msg {
					return ui.SetBreadcrumbMsg{Crumbs: []string{"DynamoDB", tableName}}
				},
			)
		}
	}

	return m, nil
}

// IsTextInputActive implements ui.TextInputActive. It returns true while the
// user is typing a filter so that app.go passes all keys straight through
// instead of intercepting global shortcuts.
func (m TablesModel) IsTextInputActive() bool { return m.filtering }

// visible returns tables matching the current filter.
func (m *TablesModel) visible() []string {
	if m.filter == "" {
		return m.tables
	}
	var out []string
	f := strings.ToLower(m.filter)
	for _, t := range m.tables {
		if strings.Contains(strings.ToLower(t), f) {
			out = append(out, t)
		}
	}
	return out
}

// isCredentialError returns true when err looks like an AWS credential failure.
func isCredentialError(err error) bool {
	if err == nil {
		return false
	}
	msg := err.Error()
	return strings.Contains(msg, "get credentials") ||
		strings.Contains(msg, "no credentials") ||
		strings.Contains(msg, "failed to refresh cached credentials") ||
		strings.Contains(msg, "NoCredentialProviders")
}

func (m TablesModel) View() string {
	if m.loading {
		content := fmt.Sprintf("%s  Loading tables…", m.spinner.View())
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}
	if m.err != nil {
		if isCredentialError(m.err) {
			content := ui.StyleDanger.Background(ui.ColorBg).Render("No AWS credentials found.") +
				ui.StyleMuted.Background(ui.ColorBg).Render("\n\nMake sure a profile is configured in ~/.aws/config,\nor set AWS_PROFILE / AWS_ACCESS_KEY_ID in your environment.\n\nRun \"aws configure\" to set up credentials.") +
				ui.StyleDimmed.Background(ui.ColorBg).Render("\n\nr  retry   p  switch profile")
			return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
				lipgloss.WithWhitespaceBackground(ui.ColorBg))
		}
		content := ui.StyleDanger.Background(ui.ColorBg).Render(m.err.Error()) +
			ui.StyleDimmed.Background(ui.ColorBg).Render("\n\nr  retry")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	vis := m.visible()
	total := len(m.tables)

	// Header line — full terminal width.
	title := ui.StyleTitle.Background(ui.ColorBg).Render("DynamoDB")
	count := ui.StyleCount.Render(fmt.Sprintf("  %d / %d tables", len(vis), total))
	filterHint := ""
	if m.filtering {
		filterHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.filter) +
			ui.StyleCount.Render("█")
	} else if m.filter != "" {
		filterHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.filter)
	}
	header := ui.StyleItemsHeader.Width(m.width).Render(title + count + filterHint)

	// Separator — full terminal width.
	sep := ui.HorizontalSep(m.width)

	// Table rows — each row at full terminal width.
	maxRows := m.height - 4
	if maxRows < 1 {
		maxRows = 1
	}

	// Scroll window
	start := 0
	if m.cursor >= maxRows {
		start = m.cursor - maxRows + 1
	}
	end := start + maxRows
	if end > len(vis) {
		end = len(vis)
	}

	var rows strings.Builder
	for i := start; i < end; i++ {
		t := vis[i]
		if i == m.cursor {
			hlSp := lipgloss.NewStyle().Background(ui.ColorHighlight).Render("  ")
			row := ui.StyleListRowSelected.Width(m.width).Render(
				hlSp +
					ui.StyleListRowSelectedCursor.Render("›") +
					hlSp +
					ui.StyleListRowSelectedName.Render(t),
			)
			rows.WriteString(row + "\n")
		} else {
			row := ui.StyleListRowNormal.Width(m.width).Render(
				"     " + ui.StyleListRowNormalName.Render(t),
			)
			rows.WriteString(row + "\n")
		}
	}

	if len(vis) == 0 && !m.filtering {
		content := ui.StyleEmptyState.Render("no tables found")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	} else if len(vis) == 0 {
		rows.WriteString(ui.StyleEmptyState.PaddingLeft(5).Render("no tables found\n"))
	}

	// Footer hints bar — full terminal width.
	var pairs [][2]string
	if m.filtering {
		pairs = [][2]string{{"esc", "cancel filter"}, {"enter", "confirm"}}
	} else {
		pairs = [][2]string{{"↑/↓", "navigate"}, {"enter", "open"}, {"/", "filter"}, {"r", "refresh"}}
	}
	hints := ui.RenderHints(m.width, pairs)

	// Pad the row area so the hints bar is always pinned to the bottom.
	// header(1) + sep(1) + hints(1) = 3 fixed lines; rest is for rows.
	rowAreaHeight := m.height - 3
	if rowAreaHeight < 1 {
		rowAreaHeight = 1
	}
	rowLines := strings.Count(rows.String(), "\n")
	for i := rowLines; i < rowAreaHeight; i++ {
		rows.WriteString(ui.StyleListRowNormal.Width(m.width).Render("") + "\n")
	}

	return header + "\n" + sep + "\n" + rows.String() + hints
}

// fetchTablesCmd fetches all DynamoDB table names via paginated ListTables.
func fetchTablesCmd(cfg aws.Config) tea.Cmd {
	return func() tea.Msg {
		client := dynamodb.NewFromConfig(cfg)
		var tables []string
		var lastEvaluated *string

		for {
			input := &dynamodb.ListTablesInput{}
			if lastEvaluated != nil {
				input.ExclusiveStartTableName = lastEvaluated
			}
			out, err := client.ListTables(context.Background(), input)
			if err != nil {
				return awspkg.ErrMsg{Err: fmt.Errorf("ListTables: %w", err)}
			}
			tables = append(tables, out.TableNames...)
			if out.LastEvaluatedTableName == nil {
				break
			}
			lastEvaluated = out.LastEvaluatedTableName
		}

		return tablesLoadedMsg{tables: tables}
	}
}
