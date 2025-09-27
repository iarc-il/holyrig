# JSON-RPC Protocol Specification

This document describes the JSON-RPC 2.0 based protocol used for communicating with radio rigs through HolyRig.

## Overview

The protocol enables clients to:
- Query available rig capabilities
- Execute rig commands
- Subscribe to and receive status updates

All communication is done using JSON-RPC 2.0 format over UDP sockets.

## Methods

### get_capabilities

Retrieves the available commands and status fields for the connected rig.

Request:
```json
{
    "jsonrpc": "2.0",
    "method": "get_capabilities",
    "id": 1
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "commands": {
            "set_freq": {
                "parameters": {
                    "freq": "number"
                }
            },
            "set_mode": {
                "parameters": {
                    "mode": "string",
                    "filter": "string"
                }
            }
        },
        "status_fields": {
            "freq": "number",
            "mode": "string",
            "filter": "string",
            "signal_strength": "number"
        }
    }
}
```

### execute_command

Executes a command on the rig.

Request:
```json
{
    "jsonrpc": "2.0",
    "method": "execute_command",
    "params": {
        "command": "set_freq",
        "parameters": {
            "freq": 14250000
        }
    },
    "id": 2
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 2,
    "result": {
        "success": true
    }
}
```

### subscribe_status

Subscribes to status updates for specified fields.

Request:
```json
{
    "jsonrpc": "2.0",
    "method": "subscribe_status",
    "params": {
        "fields": ["freq", "mode", "transmit"]
    },
    "id": 3
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 3,
    "result": {
        "subscription_id": "sub_1234"
    }
}
```

Status Update Notification (Server -> Client):
```json
{
    "jsonrpc": "2.0",
    "method": "status_update",
    "params": {
        "subscription_id": "sub_1234",
        "updates": {
            "freq": 14250000,
            "mode": "USB",
            "transmit": false
        }
    }
}
```

## Error Handling

Errors follow the JSON-RPC 2.0 error format:

```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "error": {
        "code": -32601,
        "message": "Method not found",
        "data": {
            "details": "The requested method 'unknown_method' is not supported"
        }
    }
}
```

Common error codes:
- -32600: Invalid Request
- -32601: Method not found
- -32602: Invalid params
- -32603: Internal error
- -32000: Rig communication error
- -32001: Invalid command parameters
- -32002: Subscription error
