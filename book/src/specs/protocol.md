# JSON-RPC Protocol Specification

This document describes the JSON-RPC 2.0 based protocol used for communicating with radio rigs through HolyRig.

## Overview

The protocol enables clients to:
- Query available rig capabilities
- Execute rig commands
- Subscribe to status updates, and rig state updates

All communication is done using JSON-RPC 2.0 format over UDP sockets.

## Methods

### list_rigs

Retrieves the available rigs and their state.

Request:
```json
{
    "jsonrpc": "2.0",
    "method": "list_rigs",
    "id": 1
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "0": true,
        "1": false,
        "2": false,
    }
}
```

The result keys are the rig ids that can be used in the rest of the commands and the values are the connection state.

### get_capabilities

Retrieves the available commands and status fields for the connected rig.

Request:
```json
{
    "jsonrpc": "2.0",
    "method": "get_capabilities",
    "params": {
        "rig_id": "0",
    },
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
        "rig_id": "0",
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
        "rig_id": "0",
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
        "rig_id": "0",
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

Official JSON-PRC 2.0 error codes:
- -32700: Parse error
- -32600: Invalid Request
- -32601: Method not found
- -32602: Invalid params
- -32603: Internal error

Extended error codes:
- -32000: Rig communication error
- -32001: Invalid command parameters
- -32002: Subscription error
- -32003: Unknown rig id
