# OmniRig Implementation Analysis

This document analyzes the feasibility of implementing the OmniRig COM interface using the current `com-example` and `auto-dispatch` infrastructure.

## Current Infrastructure Capabilities

### com-example
The `com-example` crate demonstrates:
- Local COM server implementation
- Singleton pattern for COM objects
- Class factory implementation
- Multiple sub-objects (nested COM objects)
- Basic property getters and setters
- Methods with multiple parameters

### auto-dispatch
The `auto-dispatch` procedural macro provides:
- Automatic `IDispatch` implementation
- Support for properties (getters/setters) via `#[getter]` and `#[setter]` attributes
- Method dispatch with automatic parameter conversion
- DISPID-based dispatch
- Supported types:
  - `IUnknown`
  - `IDispatch`
  - `BSTR`
  - `bool` (VARIANT_BOOL)
  - Integer types: `u16`, `i16`, `u32`, `i32`, `u64`, `i64`
  - `f64` (double)

## OmniRig Interface Structure

The OmniRig library consists of:

### 1. Enumerations
- `RigParamX`: Flags representing rig parameters and commands (frequency, mode, VFO operations, etc.)
- `RigStatusX`: Rig connection states (not configured, disabled, port busy, not responding, online)

### 2. Interfaces
- `IOmniRigX`: Main interface for the OmniRig object
- `IRigX`: Interface for individual rig control
- `IPortBits`: Interface for serial port control
- `IOmniRigXEvents`: Event dispatch interface for notifications

### 3. Coclasses
- `OmniRigX`: Main automation object
- `RigX`: Rig control object
- `PortBits`: Port control object

## Implementation Feasibility

### Fully Supported Features

#### 1. Properties with Getters/Setters
All basic properties can be implemented:

**IOmniRigX Properties:**
```rust
#[auto_dispatch]
impl OmniRigX {
    #[id(0x01)]
    #[getter]
    fn InterfaceVersion(&self) -> Result<i32, HRESULT> {
        Ok(0x101) // Version 1.01
    }

    #[id(0x05)]
    #[getter]
    fn DialogVisible(&self) -> Result<bool, HRESULT> {
        Ok(*self.dialog_visible.read().unwrap())
    }

    #[id(0x05)]
    #[setter]
    fn DialogVisible(&self, value: bool) -> Result<(), HRESULT> {
        *self.dialog_visible.write().unwrap() = value;
        Ok(())
    }
}
```

**IRigX Properties:**
- `Freq`, `FreqA`, `FreqB` (frequency properties)
- `RitOffset`, `Pitch` (numeric properties)
- `Vfo`, `Split`, `Rit`, `Xit`, `Tx`, `Mode` (enum-based properties)
- `RigType`, `StatusStr` (string properties - BSTR)
- `ReadableParams`, `WriteableParams` (bit flags)
- `Status` (enum)

**IPortBits Properties:**
- `Rts`, `Dtr`, `Cts`, `Dsr` (boolean serial port signals)

#### 2. Methods with Parameters
All methods with basic parameter types are supported:

```rust
#[id(0x04)]
fn IsParamReadable(&self, param: i32) -> Result<bool, HRESULT> {
    // Check if parameter is readable
    Ok(self.readable_params.read().unwrap() & param != 0)
}

#[id(0x14)]
fn SetSimplexMode(&self, freq: i32) -> Result<(), HRESULT> {
    // Set simplex mode
    self.set_frequency(freq)?;
    self.set_split(false)?;
    Ok(())
}

#[id(0x15)]
fn SetSplitMode(&self, rx_freq: i32, tx_freq: i32) -> Result<(), HRESULT> {
    // Set split mode
    self.set_rx_frequency(rx_freq)?;
    self.set_tx_frequency(tx_freq)?;
    self.set_split(true)?;
    Ok(())
}

#[id(0x13)]
fn ClearRit(&self) -> Result<(), HRESULT> {
    *self.rit_offset.write().unwrap() = 0;
    Ok(())
}
```

#### 3. Nested COM Objects
Sub-objects can be returned as `IDispatch`:

```rust
#[id(0x03)]
#[getter]
fn Rig1(&self) -> Result<IDispatch, HRESULT> {
    self.rig1.read().unwrap().as_ref().cloned().ok_or(E_FAIL)
}

#[id(0x04)]
#[getter]
fn Rig2(&self) -> Result<IDispatch, HRESULT> {
    self.rig2.read().unwrap().as_ref().cloned().ok_or(E_FAIL)
}

// In IRigX:
#[id(0x1A)]
#[getter]
fn PortBits(&self) -> Result<IDispatch, HRESULT> {
    self.port_bits.read().unwrap().as_ref().cloned().ok_or(E_FAIL)
}
```

#### 4. Enumerations as Integer Types
Enums can be implemented and passed as `i32`:

```rust
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigParamX {
    PmUnknown = 1,
    PmFreq = 2,
    PmFreqA = 4,
    PmFreqB = 8,
    // ... all other values
}

impl From<i32> for RigParamX {
    fn from(value: i32) -> Self {
        // Conversion logic
    }
}

impl From<RigParamX> for i32 {
    fn from(value: RigParamX) -> Self {
        value as i32
    }
}
```

#### 5. Multiple COM Objects / Coclasses
The infrastructure supports registering multiple class factories:

```rust
const CLSID_OMNIRIG: GUID = GUID::from_u128(0x0839E8C6_ED30_4950_8087_966F970F0CAE);
const CLSID_RIG: GUID = GUID::from_u128(0x78AECFA2_3F52_4E39_98D3_1646C00A6234);
const CLSID_PORTBITS: GUID = GUID::from_u128(0xB786DE29_3B3D_4C66_B7C4_547F9A77A21D);

// Register multiple factories
let omnirig_factory: IClassFactory = OmniRigFactory.into();
let cookie1 = CoRegisterClassObject(&CLSID_OMNIRIG, &omnirig_factory, ...)?;

let rig_factory: IClassFactory = RigFactory.into();
let cookie2 = CoRegisterClassObject(&CLSID_RIG, &rig_factory, ...)?;

// etc.
```

### Not Supported (Current Limitations)

#### 1. Event Interfaces (Connection Points)

**Limitation:** The most significant missing feature.

```idl
dispinterface IOmniRigXEvents {
    [id(0x00000001)]
    HRESULT VisibleChange(void);
    [id(0x00000002)]
    HRESULT RigTypeChange([in] long RigNumber);
    [id(0x00000003)]
    HRESULT StatusChange([in] long RigNumber);
    [id(0x00000004)]
    HRESULT ParamsChange([in] long RigNumber, [in] long Params);
    [id(0x00000005)]
    HRESULT CustomReply([in] long RigNumber, [in] VARIANT Command, [in] VARIANT Reply);
}
```

**Why:** Event sources require implementing:
- `IConnectionPointContainer` interface
- `IConnectionPoint` interface
- Event sink management
- Thread-safe callback invocation

The current `auto-dispatch` macro only generates `IDispatch` implementations. Connection points are a separate COM pattern for events and notifications.

**Impact:**
- Clients cannot receive notifications about rig status changes
- Applications must poll for changes instead of receiving events
- This is a critical limitation for interactive applications that need real-time updates

**Workaround:** Clients would need to poll properties periodically:
```python
# Without events (polling approach)
while True:
    current_status = rig.Status
    if current_status != last_status:
        # Handle status change
        last_status = current_status
    time.sleep(0.1)
```

#### 2. VARIANT Parameters

**Limitation:** Methods using `VARIANT` type cannot be fully implemented.

```idl
HRESULT SendCustomCommand([in] VARIANT Command, [in] long ReplyLength, [in] VARIANT ReplyEnd);
```

**Why:** The `PropertyType` enum in `auto_dispatch.rs` doesn't include `VARIANT` as a supported type. `VARIANT` is a complex discriminated union that can hold:
- Numeric types (VT_I4, VT_R8, etc.)
- Strings (VT_BSTR)
- Booleans (VT_BOOL)
- Arrays (VT_ARRAY)
- Other objects (VT_DISPATCH, VT_UNKNOWN)
- And many more...

**Potential Extension:** Could be added to the macro with significant work:

```rust
// Would need to add to PropertyType enum
enum PropertyType {
    // ... existing types ...
    Variant,  // New
}

// Would need conversion implementations
impl TryFrom<&VARIANT> for VariantWrapper {
    fn try_from(variant: &VARIANT) -> Result<Self, Error> {
        unsafe {
            match variant.vt() {
                VT_I4 => Ok(VariantWrapper::I4(variant.Anonymous.Anonymous.Anonymous.lVal)),
                VT_BSTR => Ok(VariantWrapper::Bstr(/* ... */)),
                // Handle all variant types...
            }
        }
    }
}
```

**Impact:**
- `SendCustomCommand` cannot be implemented
- `CustomReply` event cannot be properly handled
- Limits flexibility for custom radio commands

#### 3. Type Library Generation

**Limitation:** No `.tlb` (Type Library) file generation.

**Why:** The IDL file is meant to be compiled into a type library that contains:
- Interface definitions
- Method signatures
- Parameter metadata
- Default interface selection (`[default]` attribute)
- Event source information (`[source]` attribute)

Our infrastructure generates implementations at compile time but doesn't create runtime type libraries.

**Impact:**
- Early binding in VBA/VBScript won't work optimally
- IntelliSense/autocomplete in some scripting environments won't work
- Some scripting clients expect type libraries for proper operation

**Workaround:** Late binding via `IDispatch` works for most scripting languages:
```python
# Late binding (works without type library)
import win32com.client
rig = win32com.client.Dispatch("{78AECFA2-3F52-4E39-98D3-1646C00A6234}")
rig.Freq = 14250000  # Will work via IDispatch
```

### Partial Support / Needs Extension

#### 1. BSTR String Type

**Status:** Defined in `PropertyType` enum but conversion implementation not visible in provided code.

**Required for:**
- `RigType` property (string)
- `StatusStr` property (string)

**Needs:**
```rust
// Conversion from VARIANT to BSTR
impl TryFrom<&VARIANT> for BSTR {
    fn try_from(variant: &VARIANT) -> Result<Self, Error> {
        // Implementation needed
    }
}

// Conversion to VARIANT
impl From<BSTR> for VARIANT {
    fn from(bstr: BSTR) -> Self {
        // Implementation needed
    }
}
```

#### 2. Default Property (DISPID 0)

**Status:** Not explicitly handled.

Some COM interfaces define a default property with `DISPID_VALUE` (0). While the infrastructure supports custom DISPIDs, special handling for DISPID 0 may be needed.

## Implementation Coverage Estimate

Based on the analysis:

| Feature Category | Support Level | Coverage |
|-----------------|---------------|----------|
| Properties (get/set) | ✅ Full | 100% |
| Methods with basic types | ✅ Full | 100% |
| Integer/Boolean types | ✅ Full | 100% |
| Nested objects | ✅ Full | 100% |
| Enumerations | ✅ Full | 100% |
| String properties (BSTR) | ⚠️ Partial | 90% |
| VARIANT parameters | ❌ None | 0% |
| Event interfaces | ❌ None | 0% |
| Type library | ❌ None | 0% |

**Overall Core Functionality: ~70-75%**

The core rig control functionality (setting frequencies, modes, reading status) can be implemented. However, real-time event notifications and advanced custom command features are not supported.

## Recommended Implementation Strategy

### Phase 1: Core Interfaces (Fully Supported)
1. Implement `IRigX` interface with all properties and basic methods
2. Implement `IPortBits` interface
3. Implement `IOmniRigX` interface with property access to Rig1/Rig2
4. Implement enumerations (`RigParamX`, `RigStatusX`)

### Phase 2: BSTR Support
1. Add/verify BSTR conversion in `auto_dispatch`
2. Implement string properties (`RigType`, `StatusStr`)

### Phase 3: Advanced Features (Requires Infrastructure Extension)
1. **Consider if worth the effort:**
   - Connection point implementation for events
   - VARIANT support for custom commands
   - Type library generation

2. **Alternative approaches:**
   - Document the polling-based approach for clients
   - Provide a wrapper library that handles polling internally
   - Consider a hybrid approach with a custom notification mechanism

## Client Compatibility

### Will Work:
- Python with `win32com.client.Dispatch()` (late binding)
- PowerShell with `New-Object -ComObject`
- Any language using `IDispatch` late binding
- Applications that poll for status changes

### May Have Issues:
- Applications expecting event notifications (would need polling)
- VBA/VBScript with early binding (no type library)
- Applications using `SendCustomCommand` with VARIANT parameters
- Real-time applications requiring immediate status updates

## Conclusion

The current infrastructure can implement approximately **70-75%** of the OmniRig interface, covering all core rig control functionality. The main limitations are:

1. **No event support** (most significant - requires architectural extension)
2. **No VARIANT parameter support** (limits custom commands)
3. **No type library generation** (reduces developer experience in some environments)

For applications that primarily need to:
- Set and read frequencies
- Change modes and VFO settings
- Read rig status
- Control basic rig operations

The implementation is **fully feasible** with the current infrastructure.

For applications requiring:
- Real-time event notifications
- Custom command sequences
- Full OmniRig compatibility

Additional infrastructure development would be required, particularly for connection point support.


