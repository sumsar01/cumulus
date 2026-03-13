package ui

import (
	"github.com/charmbracelet/lipgloss"
)

// HelpOverlay renders a keyboard shortcut reference as a centred overlay.
type HelpOverlay struct{}

// NewHelpOverlay constructs an empty HelpOverlay.
func NewHelpOverlay() HelpOverlay { return HelpOverlay{} }

// View renders the help content centred on screen.
func (h *HelpOverlay) View(width, height int) string {
	title := StyleTitle.Background(ColorSurface).Render("keyboard shortcuts")
	sep := StyleDimmed.Background(ColorSurface).Render("────────────────────────────────────────")

	global := sectionHeader("Global") +
		helpRow("q / ctrl+c", "quit") +
		helpRow("p", "switch AWS profile") +
		helpRow("?", "toggle help") +
		helpRow("esc", "go back / close overlay")

	nav := sectionHeader("Navigation") +
		helpRow("↑ / ↓  or  k / j", "move cursor") +
		helpRow("enter", "select / drill in") +
		helpRow("←  pgup  /  →  pgdn", "previous / next page")

	dynamo := sectionHeader("DynamoDB — Tables") +
		helpRow("r", "refresh") +
		helpRow("/", "filter tables") +
		helpRow("esc", "clear filter / go back")

	items := sectionHeader("DynamoDB — Items") +
		helpRow("r", "refresh") +
		helpRow("Q", "query by partition key") +
		helpRow("/", "filter expression") +
		helpRow("n", "new item") +
		helpRow("e", "edit selected item") +
		helpRow("d", "delete selected item")

	detail := sectionHeader("DynamoDB — Detail") +
		helpRow("↑ / ↓", "scroll") +
		helpRow("y", "copy JSON to clipboard")

	content := title + "\n" + sep + "\n" +
		global + sep + "\n" +
		nav + sep + "\n" +
		dynamo + sep + "\n" +
		items + sep + "\n" +
		detail + "\n" +
		StyleDimmed.Background(ColorSurface).Render("?  or  esc  to close")

	box := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(ColorBorder).
		Background(ColorSurface).
		Padding(1, 3).
		Width(54).
		Render(content)

	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ColorBg),
	)
}

func sectionHeader(s string) string {
	return "\n" + StyleSubtitle.Background(ColorSurface).Render(s) + "\n"
}

func helpRow(key, desc string) string {
	k := StyleKey.Width(22).Background(ColorSurface).Render(key)
	d := StyleMuted.Background(ColorSurface).Render(desc)
	return k + d + "\n"
}
