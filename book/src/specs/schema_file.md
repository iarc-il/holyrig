# Schema file
The schema describes the commands and data types used to communicate with the rig.
It is a generic API description that isn't specific for a rig model.

## Sections
The schema file is a toml file that has the following section:

### general
field "type", must be "transceiver". This will be used to allow future extensions.
Field "version", must be "1". This will be used to allow future extensions.

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

The string "vfo" can be used just like a regular type in command parameters.
It is up to the model file to define the actual numerical value of each member of the enum,
and it can omit unsupported members. The enum values are treated just like regular integer in a command section.

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

## Data types
These are the builtin data types:
  * `int` - an unsigned 32 bit integer
  * `bool` - boolean value
