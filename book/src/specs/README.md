# Specification Files

A core principle of Holyrig is **extensibility**.  
Users and developers should be able to define custom commands in any format they choose.

Holyrig is not strictly limited to transceiver control,
It can easily be extended to support other serial-based hardware such as antenna rotators, tuners, amplifiers,  
or any device that communicates over a serial connection.

## File Types

There are two types of configuration files: **schema files** and **model files**.

* The **schema file** defines the commands and data types used to communicate with the rig.
* The **model file** defines the binary format in which those commands that are sent.

Both schema and model files use the standard [TOML](https://toml.io/en/) format.
