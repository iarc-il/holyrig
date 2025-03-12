# Preface
This document is meant to define and organize the different tasks and decisions we have to make.

# What is Holyrig?
Holyrig is meant to be the next-generation CAT control program the will enable:
* More features of the tranciever
* More operating systems
* More communication options

## Design goals
* Full compatability with existing software
    * Should be drop-in replacement to existing software (Omniring 1.2, Omnirig 2.0, rigctl)
* Familiar graphical interface
    * Should look like Omnirig
* Extended rig commands support
* Flexible communication channels
    * Network access

# Roadmap
* First POC
    * Headless process (no UI)
    * Support setting and getting frequency
    * One rig support (probably 7300)
* Basic GUI support
    * Settings should be set only from GUI
* Omnirig feature parity
    * Support all Omnirig commands
    * Support all Omnirig settings
    * Common software works with the Holyrig (WSJT-X for example)

* Tasks
* CAT
    * Choosing between using Omnirig rig files, Hamlib or other solution.
* COM
    * Masquerading as Omnirig or adding other COM component
* GUI
    * Choosing GUI framework
