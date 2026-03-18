package main

import (
	"fmt"
	"github.com/charmbracelet/bubbles/table"
	"github.com/charmbracelet/lipgloss"
	"github.com/sumsar01/cumulus/internal/ui"
)

func main() {
	t := table.New(
		table.WithColumns([]table.Column{{Title: "Name", Width: 20}, {Title: "Value", Width: 20}}),
		table.WithRows([]table.Row{{"row one", "val1"}, {"row two", "val2"}, {"row three", "val3"}}),
		table.WithFocused(true),
		table.WithStyles(ui.DynamoTableStyles),
		table.WithHeight(5),
	)

	fmt.Printf("Selected BG set: %v\n", ui.DynamoTableStyles.Selected.GetBackground())
	fmt.Printf("Cell BG set: %v\n", ui.DynamoTableStyles.Cell.GetBackground())
	
	// Simulate what renderRow does
	cellStyle := ui.DynamoTableStyles.Cell
	innerStyle := lipgloss.NewStyle().Width(20).MaxWidth(20).Inline(true)
	cell := cellStyle.Render(innerStyle.Render("row one"))
	selectedStyle := ui.DynamoTableStyles.Selected
	fmt.Printf("Cell rendered len: %d\n", len(cell))
	fmt.Printf("Selected rendered: %q\n", selectedStyle.Render(cell))
	fmt.Printf("\nTable view:\n%s\n", t.View())
}
