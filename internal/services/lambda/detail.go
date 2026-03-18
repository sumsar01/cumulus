package lambda

import (
	"fmt"
	"sort"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	lambdasdk "github.com/aws/aws-sdk-go-v2/service/lambda"
	lambdatypes "github.com/aws/aws-sdk-go-v2/service/lambda/types"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/ui"
)

// FunctionDetailModel shows the full configuration of a single Lambda function
// in a scrollable viewport. Environment variable values are masked.
type FunctionDetailModel struct {
	cfg          aws.Config
	functionName string
	// initial snapshot from the list (shown immediately while detail loads)
	snapshot lambdatypes.FunctionConfiguration
	// full detail from GetFunction (nil until loaded)
	detail  *lambdasdk.GetFunctionOutput
	spinner spinner.Model
	loading bool
	err     error

	viewport viewport.Model
	ready    bool
	width    int
	height   int
}

// NewFunctionDetailModel constructs the detail view for a Lambda function.
// The snapshot is the FunctionConfiguration already in hand from the list view;
// it is rendered immediately so the screen is not blank while GetFunction loads.
func NewFunctionDetailModel(
	snapshot lambdatypes.FunctionConfiguration,
	cfg aws.Config,
	width, height int,
) FunctionDetailModel {
	m := FunctionDetailModel{
		cfg:          cfg,
		functionName: aws.ToString(snapshot.FunctionName),
		snapshot:     snapshot,
		spinner:      ui.NewSpinner(),
		loading:      true,
		width:        width,
		height:       height,
	}
	if width > 0 && height > 0 {
		m.initViewport(width, height)
	}
	return m
}

// Init starts loading the full function detail.
func (m FunctionDetailModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, fetchFunctionDetailCmd(m.cfg, m.functionName))
}

func (m FunctionDetailModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.initViewport(msg.Width, msg.Height)
		return m, nil

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.detail = nil
		m.initViewport(m.width, m.height)
		return m, tea.Batch(m.spinner.Tick, fetchFunctionDetailCmd(m.cfg, m.functionName))

	case functionDetailLoadedMsg:
		m.loading = false
		m.detail = msg.output
		m.initViewport(m.width, m.height)
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
		switch msg.String() {
		case "r":
			m.loading = true
			m.err = nil
			m.detail = nil
			return m, tea.Batch(m.spinner.Tick, fetchFunctionDetailCmd(m.cfg, m.functionName))
		}
	}

	if m.ready {
		vp, cmd := m.viewport.Update(msg)
		m.viewport = vp
		return m, cmd
	}
	return m, nil
}

// ── View ──────────────────────────────────────────────────────────────────────

func (m FunctionDetailModel) View() string {
	title := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width).Render(
		ui.StyleTitle.Background(ui.ColorBg).Render(m.functionName) +
			ui.StyleMuted.Background(ui.ColorBg).Render("  — function detail"),
	)
	sep := ui.HorizontalSep(m.width)

	if m.err != nil {
		content := ui.StyleDanger.Background(ui.ColorBg).Render(m.err.Error()) +
			ui.StyleDimmed.Background(ui.ColorBg).Render("\n\nr  retry   esc  back")
		body := lipgloss.Place(m.width, m.height-2, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
		return title + "\n" + sep + "\n" + body
	}

	if !m.ready {
		content := ui.StyleMuted.Background(ui.ColorBg).Render("  Initialising…")
		body := lipgloss.Place(m.width, m.height-2, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
		return title + "\n" + sep + "\n" + body
	}

	// Loading indicator overlaid on the header when refreshing.
	loadingStr := ""
	if m.loading {
		loadingStr = "  " + m.spinner.View()
	}

	scrollPct := fmt.Sprintf("  %3.f%%", m.viewport.ScrollPercent()*100)
	hints := lipgloss.NewStyle().Background(ui.ColorSurface).Width(m.width).Render(
		ui.StyleMuted.Background(ui.ColorSurface).Render("↑/↓ scroll  r refresh  esc back"+loadingStr) +
			ui.StyleMuted.Background(ui.ColorSurface).Render(scrollPct),
	)

	lineStyle := lipgloss.NewStyle().Background(ui.ColorBg).Width(m.width)
	vpLines := strings.Split(m.viewport.View(), "\n")
	for i, line := range vpLines {
		vpLines[i] = lineStyle.Render(line)
	}

	return title + "\n" + sep + "\n" + strings.Join(vpLines, "\n") + "\n" + hints
}

// ── Viewport helpers ──────────────────────────────────────────────────────────

func (m *FunctionDetailModel) initViewport(width, height int) {
	headerH := 2 // title + sep
	footerH := 1 // hints
	vp := viewport.New(width, height-headerH-footerH)
	vp.Style = ui.StyleViewportBorder
	vp.SetContent(m.renderContent())
	m.viewport = vp
	m.ready = true
}

// renderContent builds the styled text content for the viewport.
func (m *FunctionDetailModel) renderContent() string {
	var b strings.Builder

	// Use full detail if loaded, otherwise fall back to the snapshot.
	cfg := m.snapshot
	if m.detail != nil && m.detail.Configuration != nil {
		cfg = *m.detail.Configuration
	}

	kStyle := lipgloss.NewStyle().Foreground(ui.ColorAccent).Background(ui.ColorBg).Bold(true)
	vStyle := lipgloss.NewStyle().Foreground(ui.ColorText).Background(ui.ColorBg)
	muted := lipgloss.NewStyle().Foreground(ui.ColorMuted).Background(ui.ColorBg)
	section := lipgloss.NewStyle().Foreground(ui.ColorPrimary).Background(ui.ColorBg).Bold(true)
	masked := lipgloss.NewStyle().Foreground(ui.ColorMuted).Background(ui.ColorBg)

	row := func(key, value string) {
		b.WriteString(kStyle.Render(padRight(key, 22)) + vStyle.Render(value) + "\n")
	}

	// ── General ───────────────────────────────────────────────────────────────
	b.WriteString(section.Render("General") + "\n")
	b.WriteString(muted.Render(strings.Repeat("─", 40)) + "\n")
	row("ARN", aws.ToString(cfg.FunctionArn))
	row("Description", nonEmpty(aws.ToString(cfg.Description)))
	row("Runtime", string(cfg.Runtime))
	row("Handler", aws.ToString(cfg.Handler))
	row("Package type", string(cfg.PackageType))
	row("Architectures", joinArchs(cfg.Architectures))
	row("Code size", fmt.Sprintf("%d bytes", cfg.CodeSize))
	row("Last modified", aws.ToString(cfg.LastModified))
	b.WriteString("\n")

	// ── Resources ─────────────────────────────────────────────────────────────
	b.WriteString(section.Render("Resources") + "\n")
	b.WriteString(muted.Render(strings.Repeat("─", 40)) + "\n")
	if cfg.MemorySize != nil {
		row("Memory", fmt.Sprintf("%d MB", *cfg.MemorySize))
	}
	if cfg.Timeout != nil {
		row("Timeout", fmt.Sprintf("%ds", *cfg.Timeout))
	}
	if cfg.EphemeralStorage != nil {
		row("Ephemeral storage", fmt.Sprintf("%d MB", aws.ToInt32(cfg.EphemeralStorage.Size)))
	}
	b.WriteString("\n")

	// ── Permissions ───────────────────────────────────────────────────────────
	b.WriteString(section.Render("Permissions") + "\n")
	b.WriteString(muted.Render(strings.Repeat("─", 40)) + "\n")
	row("Role", aws.ToString(cfg.Role))
	if cfg.KMSKeyArn != nil {
		row("KMS key ARN", aws.ToString(cfg.KMSKeyArn))
	}
	b.WriteString("\n")

	// ── VPC ───────────────────────────────────────────────────────────────────
	if cfg.VpcConfig != nil && aws.ToString(cfg.VpcConfig.VpcId) != "" {
		b.WriteString(section.Render("VPC") + "\n")
		b.WriteString(muted.Render(strings.Repeat("─", 40)) + "\n")
		row("VPC ID", aws.ToString(cfg.VpcConfig.VpcId))
		row("Subnets", strings.Join(cfg.VpcConfig.SubnetIds, ", "))
		row("Security groups", strings.Join(cfg.VpcConfig.SecurityGroupIds, ", "))
		b.WriteString("\n")
	}

	// ── Environment variables (keys only, values masked) ─────────────────────
	if cfg.Environment != nil && len(cfg.Environment.Variables) > 0 {
		b.WriteString(section.Render(fmt.Sprintf("Environment Variables (%d)", len(cfg.Environment.Variables))) + "\n")
		b.WriteString(muted.Render(strings.Repeat("─", 40)) + "\n")
		keys := make([]string, 0, len(cfg.Environment.Variables))
		for k := range cfg.Environment.Variables {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		for _, k := range keys {
			b.WriteString(
				kStyle.Render(padRight(k, 30)) +
					masked.Render("****") + "\n",
			)
		}
		b.WriteString("\n")
	}

	// ── Layers ────────────────────────────────────────────────────────────────
	if len(cfg.Layers) > 0 {
		b.WriteString(section.Render(fmt.Sprintf("Layers (%d)", len(cfg.Layers))) + "\n")
		b.WriteString(muted.Render(strings.Repeat("─", 40)) + "\n")
		for _, l := range cfg.Layers {
			b.WriteString(vStyle.Render("  "+aws.ToString(l.Arn)) + "\n")
		}
		b.WriteString("\n")
	}

	// ── Tags (only available in full GetFunction response) ────────────────────
	if m.detail != nil && len(m.detail.Tags) > 0 {
		b.WriteString(section.Render(fmt.Sprintf("Tags (%d)", len(m.detail.Tags))) + "\n")
		b.WriteString(muted.Render(strings.Repeat("─", 40)) + "\n")
		tagKeys := make([]string, 0, len(m.detail.Tags))
		for k := range m.detail.Tags {
			tagKeys = append(tagKeys, k)
		}
		sort.Strings(tagKeys)
		for _, k := range tagKeys {
			b.WriteString(kStyle.Render(padRight(k, 30)) + vStyle.Render(m.detail.Tags[k]) + "\n")
		}
		b.WriteString("\n")
	}

	return b.String()
}

// ── Small helpers ─────────────────────────────────────────────────────────────

func nonEmpty(s string) string {
	if s == "" {
		return "—"
	}
	return s
}

func joinArchs(archs []lambdatypes.Architecture) string {
	if len(archs) == 0 {
		return "—"
	}
	parts := make([]string, len(archs))
	for i, a := range archs {
		parts[i] = string(a)
	}
	return strings.Join(parts, ", ")
}
