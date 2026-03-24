package lambda

import (
	"fmt"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	lambdatypes "github.com/aws/aws-sdk-go-v2/service/lambda/types"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/ui"
)

// LambdasModel lists Lambda functions with pagination and name filtering.
type LambdasModel struct {
	cfg       aws.Config
	functions []lambdatypes.FunctionConfiguration
	nextToken *string // non-nil when more pages are available
	cursor    int
	filter    string
	filtering bool
	spinner   spinner.Model
	loading   bool
	err       error
	width     int
	height    int
}

// NewLambdasModel constructs the Lambda functions list view.
func NewLambdasModel(cfg aws.Config) LambdasModel {
	return LambdasModel{cfg: cfg, spinner: ui.NewSpinner(), loading: true}
}

func (m LambdasModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, fetchFunctionsCmd(m.cfg, nil))
}

func (m LambdasModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.functions = nil
		m.nextToken = nil
		m.cursor = 0
		m.filter = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchFunctionsCmd(m.cfg, nil))

	case awspkg.RegionChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.functions = nil
		m.nextToken = nil
		m.cursor = 0
		m.filter = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchFunctionsCmd(m.cfg, nil))

	case functionsLoadedMsg:
		m.loading = false
		m.functions = append(m.functions, msg.functions...)
		m.nextToken = msg.nextToken
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

		// Normal keybindings.
		switch msg.String() {
		case "/":
			m.filtering = true
		case "r":
			m.loading = true
			m.err = nil
			m.functions = nil
			m.nextToken = nil
			m.cursor = 0
			m.filter = ""
			m.filtering = false
			return m, tea.Batch(m.spinner.Tick, fetchFunctionsCmd(m.cfg, nil))
		case "n":
			// Load next page if available.
			if m.nextToken != nil {
				m.loading = true
				token := m.nextToken
				m.nextToken = nil
				return m, tea.Batch(m.spinner.Tick, fetchFunctionsCmd(m.cfg, token))
			}
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
			fn := vis[m.cursor]
			detail := NewFunctionDetailModel(fn, m.cfg, m.width, m.height)
			name := aws.ToString(fn.FunctionName)
			return m, tea.Batch(
				detail.Init(),
				func() tea.Msg { return ui.PushMsg{Model: detail} },
				func() tea.Msg {
					return ui.SetBreadcrumbMsg{Crumbs: []string{"Lambda", name}}
				},
			)
		}
	}

	return m, nil
}

// IsTextInputActive implements ui.TextInputActive so that the app does not
// intercept global shortcuts while the user is typing a filter.
func (m LambdasModel) IsTextInputActive() bool { return m.filtering }

// visible returns functions matching the current filter.
func (m *LambdasModel) visible() []lambdatypes.FunctionConfiguration {
	if m.filter == "" {
		return m.functions
	}
	f := strings.ToLower(m.filter)
	var out []lambdatypes.FunctionConfiguration
	for _, fn := range m.functions {
		if strings.Contains(strings.ToLower(aws.ToString(fn.FunctionName)), f) {
			out = append(out, fn)
		}
	}
	return out
}

// ── View ──────────────────────────────────────────────────────────────────────

func (m LambdasModel) View() string {
	if m.loading && len(m.functions) == 0 {
		content := fmt.Sprintf("%s  Loading functions…", m.spinner.View())
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}
	if m.err != nil {
		content := ui.StyleDanger.Background(ui.ColorBg).Render(m.err.Error()) +
			ui.StyleDimmed.Background(ui.ColorBg).Render("\n\nr  retry")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	vis := m.visible()
	total := len(m.functions)

	// ── Header ────────────────────────────────────────────────────────────────
	title := ui.StyleTitle.Background(ui.ColorBg).Render("Lambda")
	count := ui.StyleCount.Render(fmt.Sprintf("  %d / %d functions", len(vis), total))
	filterHint := ""
	if m.filtering {
		filterHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.filter) +
			ui.StyleCount.Render("█")
	} else if m.filter != "" {
		filterHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.filter)
	}
	moreHint := ""
	if m.nextToken != nil {
		moreHint = "  " + ui.StyleCount.Render("(more available — press n)")
	}
	header := ui.StyleItemsHeader.Width(m.width).Render(title + count + filterHint + moreHint)

	sep := ui.HorizontalSep(m.width)

	// ── Column widths ─────────────────────────────────────────────────────────
	// NAME (dynamic) | RUNTIME (12) | MEMORY (9) | MODIFIED (12)
	const runtimeW, memoryW, modifiedW = 14, 9, 12
	nameW := m.width - runtimeW - memoryW - modifiedW - 6 // 6 for cursor + padding
	if nameW < 10 {
		nameW = 10
	}

	// ── Column header row ─────────────────────────────────────────────────────
	colHeader := ui.StyleDimmed.Background(ui.ColorBg).Render(
		"  " +
			padRight("  NAME", nameW+4) +
			padRight("RUNTIME", runtimeW) +
			padRight("MEMORY", memoryW) +
			"MODIFIED",
	)

	// ── Rows ──────────────────────────────────────────────────────────────────
	maxRows := m.height - 6 // header(1) + sep(1) + colHeader(1) + hints(1) + loader(1) + padding(1)
	if maxRows < 1 {
		maxRows = 1
	}

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
		fn := vis[i]
		name := truncate(aws.ToString(fn.FunctionName), nameW)
		runtime := string(fn.Runtime)
		if runtime == "" {
			runtime = "—"
		}
		memory := "—"
		if fn.MemorySize != nil {
			memory = fmt.Sprintf("%d MB", *fn.MemorySize)
		}
		modified := truncate(aws.ToString(fn.LastModified), 10) // "YYYY-MM-DD"

		if i == m.cursor {
			hlSp := lipgloss.NewStyle().Background(ui.ColorHighlight).Render("  ")
			row := ui.StyleListRowSelected.Width(m.width).Render(
				hlSp +
					ui.StyleListRowSelectedCursor.Render("›") +
					hlSp +
					ui.StyleListRowSelectedName.Render(padRight(name, nameW)) +
					lipgloss.NewStyle().Foreground(ui.ColorText).Background(ui.ColorHighlight).Render(padRight(runtime, runtimeW)) +
					lipgloss.NewStyle().Foreground(ui.ColorText).Background(ui.ColorHighlight).Render(padRight(memory, memoryW)) +
					lipgloss.NewStyle().Foreground(ui.ColorSubtext).Background(ui.ColorHighlight).Render(modified),
			)
			rows.WriteString(row + "\n")
		} else {
			row := ui.StyleListRowNormal.Width(m.width).Render(
				"     " +
					ui.StyleListRowNormalName.Render(padRight(name, nameW)) +
					ui.StyleDimmed.Background(ui.ColorBg).Render(padRight(runtime, runtimeW)) +
					ui.StyleDimmed.Background(ui.ColorBg).Render(padRight(memory, memoryW)) +
					ui.StyleDimmed.Background(ui.ColorBg).Render(modified),
			)
			rows.WriteString(row + "\n")
		}
	}

	if len(vis) == 0 && !m.filtering {
		content := ui.StyleEmptyState.Render("no functions found")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	} else if len(vis) == 0 {
		rows.WriteString(ui.StyleEmptyState.PaddingLeft(5).Render("no functions match filter\n"))
	}

	// ── Inline loading row when fetching next page ────────────────────────────
	loadingRow := ""
	if m.loading && len(m.functions) > 0 {
		loadingRow = ui.StyleDimmed.Background(ui.ColorBg).
			Render(fmt.Sprintf("  %s  Loading more…", m.spinner.View())) + "\n"
	}

	// ── Footer hints ──────────────────────────────────────────────────────────
	var pairs [][2]string
	if m.filtering {
		pairs = [][2]string{{"esc", "cancel filter"}, {"enter", "confirm"}}
	} else {
		pairs = [][2]string{
			{"↑/↓", "navigate"},
			{"enter", "detail"},
			{"/", "filter"},
			{"r", "refresh"},
		}
		if m.nextToken != nil {
			pairs = append(pairs, [2]string{"n", "load more"})
		}
	}
	hints := ui.RenderHints(m.width, pairs)

	// ── Pad rows to pin hints to bottom ──────────────────────────────────────
	// Fixed lines: header(1) + sep(1) + colHeader(1) + hints(1) = 4
	// Plus loadingRow if present.
	fixedLines := 4
	if loadingRow != "" {
		fixedLines++
	}
	rowAreaHeight := m.height - fixedLines
	if rowAreaHeight < 1 {
		rowAreaHeight = 1
	}
	rowLines := strings.Count(rows.String(), "\n")
	for i := rowLines; i < rowAreaHeight; i++ {
		rows.WriteString(ui.StyleListRowNormal.Width(m.width).Render("") + "\n")
	}

	return header + "\n" + sep + "\n" + colHeader + "\n" + rows.String() + loadingRow + hints
}

// ── Helpers ───────────────────────────────────────────────────────────────────

// padRight pads s with spaces to exactly width characters (truncating if needed).
func padRight(s string, width int) string {
	if len(s) >= width {
		return s[:width]
	}
	return s + strings.Repeat(" ", width-len(s))
}

// truncate shortens s to max characters, appending "…" if truncated.
func truncate(s string, max int) string {
	if len(s) <= max {
		return s
	}
	if max <= 1 {
		return "…"
	}
	return s[:max-1] + "…"
}
