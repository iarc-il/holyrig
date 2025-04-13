# Specification files
One main principal of the holyrig is extensibility.  
The user/software developer must be able to add any command with any format that he wants.
In fact, the holyrig isn't strictly a transciever software,
And it is quite possible to extend the current command set to support antenna rotator, tuners, amplifiers,
and every other piece of hardware that communicates via serial.

## File types
There are 2 types of files: schema file and model file.
The schema describes the commands and data types used to communicate with the rig.
The model file defines the format of the commands sent to the rig's serial port.
The schema and model files are both standard toml files.

