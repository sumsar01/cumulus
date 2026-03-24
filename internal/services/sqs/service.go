// Package sqs implements the SQS service plugin for cumulus.
// It registers itself via the services registry and provides views for
// listing queues, polling and inspecting messages, and redriving DLQs.
package sqs

import (
	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
)

// Svc is the SQS service plugin. Register it in main.go with:
//
//	services.Register(sqs.Svc{})
type Svc struct{}

func (Svc) Name() string        { return "SQS" }
func (Svc) ShortName() string   { return "sqs" }
func (Svc) Description() string { return "Browse SQS queues, poll messages, and redrive DLQs" }
func (Svc) Icon() string        { return "◎" }

// Init returns the queues-list model as the entry point for the SQS service.
func (s Svc) Init(cfg aws.Config) (tea.Model, tea.Cmd) {
	m := NewQueuesModel(cfg)
	return m, m.Init()
}
