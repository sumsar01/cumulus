# cumulus

A terminal UI for AWS. Browse and manage DynamoDB tables without leaving your terminal.

![Go version](https://img.shields.io/badge/go-1.25-blue)

## Install

```sh
go install github.com/sumsar01/cumulus@latest
```

Or build from source:

```sh
git clone https://github.com/sumsar01/aws-tui
cd aws-tui
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
| `?` | Help overlay |
| `Esc` | Go back |
| `q` | Quit |

### DynamoDB — table list

| Key | Action |
|-----|--------|
| `Enter` | Open table |
| `r` | Refresh |
| `/` | Filter tables by name |

### DynamoDB — items

| Key | Action |
|-----|--------|
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

## Configuration

cumulus looks for a config file at `~/.config/cumulus/config.toml` (honouring `$XDG_CONFIG_HOME`). The file is optional — defaults are used when it does not exist.

```toml
# ~/.config/cumulus/config.toml

editor = "nvim"  # default
```

### Editor allowlist

For security, only the following editor binaries are permitted:

`nvim`, `vim`, `vi`, `nano`, `emacs`, `hx`, `micro`

The binary is validated at config load time and again at exec time. It is never passed through a shell.

## IAM permissions

cumulus needs the following DynamoDB actions:

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

## Adding services

cumulus uses a service plugin pattern. Each AWS service is a self-contained package implementing the `Service` interface (`internal/services/service.go`). To add a new service, implement the interface and register it in `main.go`:

```go
services.Register(myservice.Svc{})
```

No other files need to change.

## License

MIT
