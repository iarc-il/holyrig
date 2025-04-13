# Preface
This document describe the data model of commands and rigs.

# Goals
* The user will be able to add new rig models without changing the code
* The user will be able to define new commands for rigs without changing the code
* All data will be written in simple, text based format

# Design
There wre 2 types of files: schema file and model file.
The schema describes the commands and data types used to communicate with the rig.
The model file defines the format of the commands sent to the rig's serial port.

# Schema file

## Sections
The schema file is a toml file that has the following section:

### general
field "type", must be "tranceiver". This will be used to allow future extensions.
field "version", must be "1". This will be used to allow future extensions.

### enums
Each entry in the enum section defines an enum type that can be used in commands or responses.
The members of the enum is a list of strings and will be stored in "members" subfield.
For example, the vfo enum is defined like that:
```toml
[enums.vfo]
members = [
    "current",
    "A",
    "B",
    "unknown"
]
```

the string "vfo" can be used just like a regular type in command parameters.
It is up to the model file to define the actual numerical value of each member of the enum,
and it can omit unsupported members.
The enum values are treated just like regular integer in a command section.

### commands
In each Each entry in the commands section defines a "function" that can be called on the rig.
This sections describes only the interface that external programs needs to know in order to communicate
with the radio. The actual implementation details are defined in a model file for a specific rig.

The `param` field is a list of pairs of (name, type) that describes the data that the function receives.
For example, the command for setting frequency is defined like that:
```toml
[commands.set_freq]
params = [
    ["freq", "int"],
    ["target", "vfo"],
]
```
The name of the command is "set_freq" and it has 2 parameters.
The first parameter is the frequency that will be sent to the radio and is an integer,
and the second parameter chooses which vfo to set.

It is assumed that the command responses doesn't contain any data the should be returned to the user,
and is only used for validation. In the future the schema file might define a return value field.

An example schema that loosely describes the omnirig commands can be found at the "rig_format.toml" file.

### status
The status section defines with parameters can be read from the radio using polling commands.
The `params` field defines the available values that can be read and their type,
with the same format as the `commands.params` field. For example:
```toml
[status]
params = [
    ["freq_a", "int"],
    ["freq_b", "int"],
    ["mode", "mode"],
]
```

Enums are also allowed as data types.
The parsed numeric value is searched in the defined enum members in the model file.
TODO: The freq_a and freq_b should be defined with the enum. How to do it?

## Data types
These are the builtin data types:
  * `int` - an unsigned 32 bit integer
  * `bool` - boolean value

# Model file
The model file defines the communication protocol with one rig model.

### Binary data formats
Binary data of commands or responses can be textual or hexidecimal.
Textual format is enclosed in parenthesis. For example: "(Some textual data)".

hexidecimal data is defined in textual hex format with optional dots.
For exmaple: "AF.12.BC90.3315" is equal to "AF12BC903315".

Note that the textual data "(Data)" is equal to the hexidecimal data: "44617461".

When a command or response have values that should be built or parsed,
The unknown parts must be a `?`. Currently, it is only supported in hexidecimal data.
For example, the command "1122??44" can only have at offset 3 with length 1.

TODO: masks

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
The `format` defines how to encode or decode the integer value (available option defined below).

The optional `add` and `multiply` adds or multiplies the interger before encoding or after decoding, if present.
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
format = "bcd_little_unsigned"
```

### Data types
These are the data types in the `format` field that specifies how numeric values are converted to binary data.

*  text                    Store each digit as ascii letter
*  int_little_unsinged     Little endian unsigned integer
*  int_big_unsinged        Big endian unsigned integer
*  bcd_little_unsigned     Little endian unsigned BCD
*  bcd_big_unsigned        Big endian unsigned BCD
*  bcd_little_signed       Little endian signed BCD. The sign is in the MSB (0x00 or 0xFF)
*  bcd_big_signed          Big endian signed BCD. The sign is in the MSB (0x00 or 0xFF)
*  yaesu                   Custom yaesu format

`int` values are limited to 32 bits. `bool` values are treated as 1 for `true` and 0 for `false`.
Enum types are converted to the numerical values specified in the model file.


## Sections
The model file is a toml file that has the following sections:

### general
The general section has 2 fields:
  * `type` field, must match the field "type" value in the schema file.
  * `version` field, must match the field "version" value in the schema file.

### enums
Each entry implements an enum in the schema file. The entry will be a dict with one field, "values"
that contains a list of pairs. Each pair has the format `[enum_member, value]` and assigns a value to the enum's
members that is specified in the schema.

### init
The init is a list of dicts and each one of the define a single initialization command that
sets the rig to the required state.
