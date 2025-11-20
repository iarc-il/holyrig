# Schema Parser

The schema parser provides a more structured and readable syntax for defining schemas compared to the legacy TOML format. This new format uses a syntax similar to rig files for consistency.

## Syntax

### Basic Structure

```rust
version = 1

schema SchemaName {
    // Enum definitions
    enum EnumName {
        Variant1,
        Variant2,
    }

    // Command declarations
    fn command_name(param_type param_name);
    fn another_command();

    // Status field definitions
    status {
        field_type field_name;
    }
}
```

### Version Declaration

Every schema file must start with a version declaration:

```rust
version = 1
```

This specifies the schema format version and must be `1` for the current implementation.

### Schema Block

The main schema block defines the schema name and contains all definitions:

```rust
schema Transceiver {
    // ... definitions
}
```

The schema name should match the type used in rig file `impl` blocks (case-insensitive).

### Enum Definitions

Enums define the available variants for enumeration types:

```rust
enum Vfo {
    Current,
    A,
    B,
    Unknown,
}
```

- Enum names should be capitalized (PascalCase)
- Variant names should be capitalized (PascalCase)
- Trailing commas are optional
- Each variant is implicitly assigned an integer value by the rig file implementation

### Command Declarations

Commands define the functions that can be implemented by rig files:

```rust
fn set_freq(int freq, Vfo target);
fn clear_rit();
fn simple_command();
```

- Function names should be lowercase with underscores (snake_case)
- Parameters specify type followed by name: `type name`
- Commands with no parameters omit the parameter list
- All command declarations must end with a semicolon

### Status Block

The status block defines fields that can be queried from the rig:

```rust
status {
    int freq_a;
    int freq_b;
    Mode mode;
    bool transmit;
}
```

- Status fields specify type followed by name: `type name`
- Each field declaration must end with a semicolon
- Status fields define what variables can be set via `set_var()` calls

## Data Types

The schema parser supports these built-in data types:

- **`int`**: 32-bit integers
- **`bool`**: Boolean values (true/false)
- **Custom enums**: Any enum defined in the schema (e.g., `Vfo`, `Mode`)

## Example Schema

Here's a complete example schema for a transceiver:

```rust
version = 1

schema Transceiver {
    enum Vfo {
        Current,
        A,
        B,
        Unknown,
    }

    enum Mode {
        CWU,
        CWL,
        USB,
        LSB,
        DIGIU,
        DIGIL,
        AM,
        FM,
    }

    // Frequency control
    fn set_freq(int freq, Vfo target);
    
    // RIT/XIT control
    fn clear_rit();
    fn set_rit(bool rit);
    fn set_xit(bool xit);
    
    // VFO control
    fn vfo_equal();
    fn vfo_swap();
    fn set_vfo(Vfo rx, Vfo tx);
    
    // Mode and other settings
    fn set_mode(Mode mode);
    fn cw_pitch(int pitch);
    fn set_split(bool split);
    fn transmit(bool tx);

    status {
        Mode mode;
        int freq_a;
        int freq_b;
        Vfo vfo;
        int cw_pitch;
        bool transmit;
        bool rit;
        bool xit;
    }
}
```

## Error Handling

The schema parser provides detailed error messages for syntax errors:

- Invalid tokens
- Missing semicolons
- Incorrect parameter syntax
- Malformed enum definitions
- Missing schema blocks

All errors are reported using the same error formatting system as the rig file parser for consistency.
