# cumulus

A terminal UI for AWS. Browse and manage AWS services without leaving your terminal.

![Go version](https://img.shields.io/badge/go-1.21+-blue)

## Supported services

- **DynamoDB** — list tables, scan/query items, view detail, create/edit/delete items
- **Lambda** — list functions, view configuration and detail
- **CloudWatch Logs** — browse log groups, streams (prefix search), and log events (filter pattern)
- **SQS** — list queues, poll messages, view detail, delete messages, redrive DLQ

## Install

```sh
go install github.com/sumsar01/cumulus@latest
```

Or build from source:

```sh
git clone https://github.com/sumsar01/cumulus
cd cumulus
go build -o cumulus .
```

## Requirements

- Go 1.21+
- AWS credentials configured (SSO, environment variables, or `~/.aws/credentials`)
- A terminal with colour support

## Usage

```sh
cumulus
```

Credentials are resolved via the standard AWS SDK v2 chain — `aws sso login`, environment variables, or a named profile. No credentials are stored or logged by cumulus.

## Keybinds

### Global

| Key | Action |
|-----|--------|
| `p` | Switch AWS profile |
| `r` | Switch AWS region |
| `?` | Help overlay |
| `Esc` | Go back |
| `q` | Quit |

### DynamoDB — table list

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | Open table |
| `r` | Refresh |
| `/` | Filter tables by name |

### DynamoDB — items

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | View item detail |
| `e` | Edit item in editor |
| `n` | New item |
| `d` | Delete item |
| `/` | Filter expression |
| `Q` | Query by partition key |
| `r` | Refresh (reset to scan) |
| `pgdn` / `→` | Next page |
| `pgup` / `←` | Previous page |

### DynamoDB — item detail

| Key | Action |
|-----|--------|
| `↑` / `↓` | Scroll |
| `y` | Copy JSON to clipboard |
| `Esc` | Back |

### Lambda — function list

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | View function detail |
| `/` | Filter by name |
| `r` | Refresh |
| `n` | Load more (pagination) |

### CloudWatch Logs — log groups

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | Open log group |
| `/` | Filter by name |
| `r` | Refresh |

### CloudWatch Logs — log streams

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | Open log stream |
| `/` | Prefix search (server-side) |
| `r` | Refresh |

### CloudWatch Logs — log events

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | View event detail |
| `/` | Filter pattern |
| `r` | Refresh |
| `n` | Next page |
| `p` | Previous page |

### SQS — queue list

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | Open queue |
| `/` | Filter by name |
| `r` | Refresh |
| `n` | Next page |

### SQS — messages

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | View message detail |
| `r` | Poll for messages |
| `d` | Delete message |
| `R` | Redrive DLQ |

## Configuration

cumulus looks for a config file at `~/.config/cumulus/config.toml` (honouring `$XDG_CONFIG_HOME`). The file is optional — defaults are used when it does not exist.

```toml
# ~/.config/cumulus/config.toml

editor = "nvim"   # default
theme  = "dark"   # dark (default) or light
```

### Editor allowlist

For security, only the following editor binaries are permitted:

`nvim`, `vim`, `vi`, `nano`, `emacs`, `hx`, `micro`

The binary is validated at config load time and again at exec time. It is never passed through a shell.

## IAM permissions

### DynamoDB

```json
{
  "Effect": "Allow",
  "Action": [
    "dynamodb:ListTables",
    "dynamodb:DescribeTable",
    "dynamodb:Scan",
    "dynamodb:Query",
    "dynamodb:PutItem",
    "dynamodb:DeleteItem"
  ],
  "Resource": "*"
}
```

### Lambda

```json
{
  "Effect": "Allow",
  "Action": [
    "lambda:ListFunctions",
    "lambda:GetFunction"
  ],
  "Resource": "*"
}
```

### CloudWatch Logs

```json
{
  "Effect": "Allow",
  "Action": [
    "logs:DescribeLogGroups",
    "logs:DescribeLogStreams",
    "logs:FilterLogEvents"
  ],
  "Resource": "*"
}
```

### SQS

```json
{
  "Effect": "Allow",
  "Action": [
    "sqs:ListQueues",
    "sqs:GetQueueAttributes",
    "sqs:ReceiveMessage",
    "sqs:DeleteMessage",
    "sqs:SendMessage"
  ],
  "Resource": "*"
}
```

## Adding services

cumulus uses a service plugin pattern. Each AWS service is a self-contained package implementing the `Service` interface (`internal/services/service.go`). To add a new service, implement the interface and register it in `main.go`:

```go
services.Register(myservice.Svc{})
```

No other files need to change.

## License

MIT
