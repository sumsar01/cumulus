package sqs

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/aws"
	awssqs "github.com/aws/aws-sdk-go-v2/service/sqs"
	sqstypes "github.com/aws/aws-sdk-go-v2/service/sqs/types"
	tea "github.com/charmbracelet/bubbletea"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
)

// ── queue listing ─────────────────────────────────────────────────────────────

// fetchQueuesCmd fetches one page of SQS queue URLs.
// Pass nextToken=nil to start from the beginning.
func fetchQueuesCmd(cfg aws.Config, nextToken *string) tea.Cmd {
	return func() tea.Msg {
		client := awssqs.NewFromConfig(cfg)
		input := &awssqs.ListQueuesInput{
			MaxResults: aws.Int32(100),
		}
		if nextToken != nil {
			input.NextToken = nextToken
		}
		out, err := client.ListQueues(context.Background(), input)
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("ListQueues: %w", err)}
		}
		return queuesLoadedMsg{
			queues:    out.QueueUrls,
			nextToken: out.NextToken,
		}
	}
}

// ── message polling ───────────────────────────────────────────────────────────

// messagesReceivedMsg carries the messages returned by a ReceiveMessage call.
type messagesReceivedMsg struct {
	messages []sqstypes.Message
}

// receiveMessagesCmd calls SQS ReceiveMessage (up to 10 messages, no long poll)
// and then immediately resets each message's visibility timeout to 0 so that
// they remain available for other consumers (peek mode).
func receiveMessagesCmd(cfg aws.Config, queueURL string) tea.Cmd {
	return func() tea.Msg {
		client := awssqs.NewFromConfig(cfg)
		ctx := context.Background()

		out, err := client.ReceiveMessage(ctx, &awssqs.ReceiveMessageInput{
			QueueUrl:              aws.String(queueURL),
			MaxNumberOfMessages:   10,
			WaitTimeSeconds:       0,
			MessageAttributeNames: []string{"All"},
			AttributeNames:        []sqstypes.QueueAttributeName{"All"},
		})
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("ReceiveMessage: %w", err)}
		}
		if len(out.Messages) == 0 {
			return messagesReceivedMsg{messages: nil}
		}

		// Peek mode: reset visibility to 0 so messages reappear immediately.
		for _, msg := range out.Messages {
			_, _ = client.ChangeMessageVisibility(ctx, &awssqs.ChangeMessageVisibilityInput{
				QueueUrl:          aws.String(queueURL),
				ReceiptHandle:     msg.ReceiptHandle,
				VisibilityTimeout: 0,
			})
		}

		return messagesReceivedMsg{messages: out.Messages}
	}
}

// ── message delete ────────────────────────────────────────────────────────────

// messageDeletedMsg signals a successful DeleteMessage call.
type messageDeletedMsg struct{}

// deleteMessageCmd deletes a single SQS message using its ReceiptHandle.
func deleteMessageCmd(cfg aws.Config, queueURL string, receiptHandle string) tea.Cmd {
	return func() tea.Msg {
		client := awssqs.NewFromConfig(cfg)
		_, err := client.DeleteMessage(context.Background(), &awssqs.DeleteMessageInput{
			QueueUrl:      aws.String(queueURL),
			ReceiptHandle: aws.String(receiptHandle),
		})
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("DeleteMessage: %w", err)}
		}
		return messageDeletedMsg{}
	}
}

// ── DLQ redrive ───────────────────────────────────────────────────────────────

// redriveInfoMsg carries the ARN information needed to confirm and start a
// DLQ redrive. The source queue ARN is extracted from the DLQ's
// RedriveAllowPolicy (if set) or the source queue's RedrivePolicy.
type redriveInfoMsg struct {
	// sourceQueueArn is the queue messages will be moved back to.
	sourceQueueArn string
	// dlqArn is the ARN of this DLQ (the move task source).
	dlqArn string
}

// redrivePolicy is the JSON structure of the SQS RedrivePolicy attribute on
// the *source* queue, which names the DLQ.
type redrivePolicy struct {
	DeadLetterTargetArn string `json:"deadLetterTargetArn"`
	MaxReceiveCount     int    `json:"maxReceiveCount"`
}

// redriveAllowPolicy is the JSON structure of the RedriveAllowPolicy attribute
// on the DLQ, which lists the source queues allowed to redrive into it.
type redriveAllowPolicy struct {
	RedrivePermission string   `json:"redrivePermission"`
	SourceQueueArns   []string `json:"sourceQueueArns"`
}

// redriveStartedMsg signals that StartMessageMoveTask was called successfully.
type redriveStartedMsg struct {
	taskHandle string
}

// getQueueAttributesCmd fetches the DLQ's ARN and its RedriveAllowPolicy so
// we can auto-detect the source queue ARN for the confirmation prompt.
func getQueueAttributesCmd(cfg aws.Config, queueURL string) tea.Cmd {
	return func() tea.Msg {
		client := awssqs.NewFromConfig(cfg)
		out, err := client.GetQueueAttributes(context.Background(), &awssqs.GetQueueAttributesInput{
			QueueUrl: aws.String(queueURL),
			AttributeNames: []sqstypes.QueueAttributeName{
				sqstypes.QueueAttributeNameQueueArn,
				sqstypes.QueueAttributeNameRedriveAllowPolicy,
			},
		})
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("GetQueueAttributes: %w", err)}
		}

		dlqArn := out.Attributes[string(sqstypes.QueueAttributeNameQueueArn)]

		// Try to read the source queue ARN from RedriveAllowPolicy.
		sourceQueueArn := ""
		if raw, ok := out.Attributes[string(sqstypes.QueueAttributeNameRedriveAllowPolicy)]; ok && raw != "" {
			var rap redriveAllowPolicy
			if err := json.Unmarshal([]byte(raw), &rap); err == nil && len(rap.SourceQueueArns) > 0 {
				sourceQueueArn = rap.SourceQueueArns[0]
			}
		}

		return redriveInfoMsg{
			sourceQueueArn: sourceQueueArn,
			dlqArn:         dlqArn,
		}
	}
}

// redriveCmd calls StartMessageMoveTask to move all messages from the DLQ
// (identified by dlqArn) back to sourceQueueArn.
func redriveCmd(cfg aws.Config, dlqArn, sourceQueueArn string) tea.Cmd {
	return func() tea.Msg {
		client := awssqs.NewFromConfig(cfg)
		input := &awssqs.StartMessageMoveTaskInput{
			SourceArn: aws.String(dlqArn),
		}
		if sourceQueueArn != "" {
			input.DestinationArn = aws.String(sourceQueueArn)
		}
		out, err := client.StartMessageMoveTask(context.Background(), input)
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("StartMessageMoveTask: %w", err)}
		}
		handle := ""
		if out.TaskHandle != nil {
			handle = *out.TaskHandle
		}
		return redriveStartedMsg{taskHandle: handle}
	}
}
