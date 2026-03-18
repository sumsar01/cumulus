// Package lambda implements the Lambda service plugin for cumulus.
// It registers itself via the services registry and provides views for
// listing Lambda functions and viewing function configuration detail.
package lambda

import (
	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
)

// Svc is the Lambda service plugin. Register it in main.go with:
//
//	services.Register(lambda.Svc{})
type Svc struct{}

func (Svc) Name() string        { return "Lambda" }
func (Svc) ShortName() string   { return "lambda" }
func (Svc) Description() string { return "Browse and inspect Lambda functions" }
func (Svc) Icon() string        { return "λ" }

// Init returns the functions-list model as the entry point for the Lambda service.
func (Svc) Init(cfg aws.Config) (tea.Model, tea.Cmd) {
	m := NewLambdasModel(cfg)
	return m, m.Init()
}
