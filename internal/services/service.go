// Package services defines the Service interface that every AWS service plugin
// must implement, and the global registry used by the navigator to enumerate
// available services.
//
// To add a new service:
//  1. Create a sub-package under internal/services/ (e.g. internal/services/s3).
//  2. Implement the Service interface.
//  3. Call services.Register(yourService{}) from an init() in main.go.
package services

import (
	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
)

// Service is the contract that every AWS service plugin must satisfy.
// Implementations should be stateless value types; Init is responsible for
// constructing the initial Bubble Tea model for that service.
type Service interface {
	// Name returns the human-readable name shown in the navigator list.
	Name() string

	// ShortName returns a short identifier used in the status bar breadcrumb.
	ShortName() string

	// Description returns a one-line description shown beneath the name.
	Description() string

	// Icon returns a single-character (or short emoji-free string) icon shown
	// in the navigator list alongside the service name.
	Icon() string

	// Init constructs the root Bubble Tea model for this service, given the
	// current AWS configuration. The returned Cmd (if any) is run immediately
	// by the host application to kick off the first data fetch.
	Init(cfg aws.Config) (tea.Model, tea.Cmd)
}

// registry holds all registered services in insertion order.
var registry []Service

// Register adds a service to the global registry. It is safe to call from
// package-level init() functions. Duplicate registrations (same ShortName)
// are silently ignored.
func Register(s Service) {
	for _, existing := range registry {
		if existing.ShortName() == s.ShortName() {
			return
		}
	}
	registry = append(registry, s)
}

// All returns a copy of the registered service slice in registration order.
func All() []Service {
	out := make([]Service, len(registry))
	copy(out, registry)
	return out
}
