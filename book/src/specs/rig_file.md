# Rig file
The rig file defines the communication protocol with one rig model.

## File name
The file name is the model name that will be displayed in the GUI, with the `.toml` extension.
For example: 'IC-7300.toml'.

## Binary data formats
Binary data of commands or responses can be textual or hexadecimal.
Textual format is enclosed in parenthesis. For example: "(Some textual data)".

Hexadecimal data is defined in textual hex format with optional dots.
For example: "AF.12.BC90.3315" is equal to "AF12BC903315".

Note that the textual data "(Data)" is equal to the hexadecimal data: "44617461".

When a command or response have values that should be built or parsed,
The unknown parts must be a `?`. Currently, it is only supported in hexadecimal data.
For example, the command "1122??44" can only have at offset 3 with length 1.

## Command format
The `init`, `command` and `status` sections all define binary format of commands and their responses.
They share a common format of command building and response parsing.

### command
The command field describes the data that will be send to the rig.

The `reply_length` defines the expected data length of the reply.
The `reply_end` defines a delimiter that ends the message and it is mutually exclusive with `reply_length`.

### Value building / parsing
Building or parsing commands and values is done with unified format.
The fields define where to insert or extract the data, in which length and at which parsing format.

The `index` field defines the starting index of the data (0 based).

The `length` field defines the length of the data in bytes.
The `length` can be ommitted when the `index` points at the start of a question mark sequence.
Then, the length can be inferred from the length of the sequence.

The `format` defines how to encode or decode the integer value (available option defined below).

The optional `add` and `multiply` adds or multiplies the integer before encoding or after decoding, if present.
First the `add` value is applies and then the `multiply`.
The values can be float or negative, but the result is always rounded to integer.

The key of the dict is the parameter to build or parse.
For example:

```toml
[commands.set_freq]
command = "1122.33.????????"
[commands.set_freq.params.freq]
add = 100
multiply = 1000
index = 3
length = 4
format = "bcd_lu"
```

### Validation
A command can have a field named `validate` that defines a mask that being matched over the received data.
The mask has the same format as a command with missing values ("AA.BB.??.DD").
The received response from the rig will be matched against the mask and raise an error if needed.

The `validate` field cannot be used with `reply_length` or `reply_end`,
since it already inherently defines this values.

### Data types
These are the data types in the `format` field that specifies how numeric values are converted to binary data.

| Format   | Meaning                                                          |
|----------|------------------------------------------------------------------|
| `bcd_bs` | Big endian signed BCD. The sign is in the MSB (0x00 or 0xFF)     |
| `bcd_bu` | Big endian unsigned BCD                                          |
| `bcd_ls` | Little endian signed BCD. The sign is in the MSB (0x00 or 0xFF)  |
| `bcd_lu` | Little endian unsigned BCD                                       |
| `int_bu` | Big endian unsigned integer                                      |
| `int_lu` | Little endian unsigned integer                                   |
| `text`   | Store each digit as ASCII letter                                 |
| `yaesu`  | Maybe will be supported in the future                            |


For example:

Value:   |     418     |    -418
---------|-------------|------------
`bcd_bs` | 00.00.04.18 | FF.00.04.18
`bcd_bu` | 00.00.04.18 | -
`bcd_ls` | 18.04.00.00 | 18.04.00.FF
`bcd_lu` | 18.04.00.00 | -
`int_bs` | 00.00.01.A2 | FF.FF.FE.5E
`int_bu` | 00.00.01.A2 | -
`int_ls` | A2.01.00.00 | 5E.FE.FF.FF
`int_lu` | A2.01.00.00 | -
`text`   | 30.34.31.38 | 2D.34.31.38

`int` values are limited to 32 bits. `bool` values are treated as 1 for `true` and 0 for `false`.
Enum types are converted to the numerical values specified in the rig file.

## Sections
The rig file is a `.toml` file that has the following sections:

### general
The general section has 2 fields:
  * `type` field, must match the field "type" value in the schema file.
  * `version` field, must match the field "version" value in the schema file.

### enums
Each entry implements an enum in the schema file. The entry will be a dict with one field, "values"
that contains a list of pairs. Each pair has the format `[enum_member, value]` and assigns a value to the enum's
members that is specified in the schema.

### init
The `init` is a list of dicts and each one of the define a single initialization command that
sets the rig to the required state.

### command
The `command` section defines a dict of command according to the commnads defined in a schema file.
Commands that are defined in the schema files can be ommitted if the rig doesn't support them.
All commands must appear in the schema file, and "custom" commands are not allowed.
