# Preface
This document describe the data model of commands and rigs.

# Goals
* The user will be able to add new rig models without changing the code
* The user will be able to define new commands for rigs without changing the code
* All data will be written in simple, text based format

# Design
There wre 2 types of files: rig schema and rig description.
The rig schema describes the commands and data types used to communicate with the rig.
The rig description defines the format of the commands sent to the rig's serial port.

# Rig schema
Rig schema is a toml file and has the following format:
* general section
    * field "type", must be "rig". This will be used to allow future extensions.
    * field "version", must be "1". This will be used to allow future extensions.
* enums section
    * Each entry in the enum section defines an enum type that can be used in commands or responses.
      the members of the enum is a list of strings and will be stored in "members" subfield.
      for example, the "mode" enum will have fields like CW, SSB, AM and FM.
* commands section
    * In each Each entry in the commands section defines a "function" that can be called on the rig.
      The param field is a list of pairs of (name, type) that describes the data that the function receives.
      For example, the "set_freq" command could have a params field with that equals [ ["freq", "int"] ]
      TODO: Command responses

Currently, The parameter types are: "int", "bool" and enum types.
And example rig schema that loosely describes the omnirig commands can be found at the "rig_format.toml" file.

# Rig description
TODO
