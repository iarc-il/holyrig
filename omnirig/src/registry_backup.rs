use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use windows::core::GUID;
use windows::Win32::System::Registry::REG_VALUE_TYPE;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteKeyExW, RegDeleteTreeW, RegEnumKeyExW, RegEnumValueW,
    RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, KEY_READ,
    KEY_WOW64_32KEY, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows_core::PCWSTR;

const OMNIRIG_CLSID: &str = "{0839E8C6-ED30-4950-8087-966F970F0CAE}";
const BACKUP_DIR: &str = "holyrig";
const BACKUP_FILE: &str = "omnirig_backup.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryValue {
    pub value_type: u32,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistryKey {
    pub values: HashMap<String, RegistryValue>,
    pub subkeys: HashMap<String, RegistryKey>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RegistryBackup {
    pub clsid_key: Option<RegistryKey>,
    pub original_exe_path: Option<String>,
    pub backup_timestamp: String,
}

fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn from_wide_string(wide: &[u16]) -> String {
    String::from_utf16_lossy(wide.split(|&c| c == 0).next().unwrap_or(&[]))
}

pub fn get_backup_file_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let appdata = env::var("APPDATA").or_else(|_| env::var("HOME"))?;
    let backup_dir = PathBuf::from(appdata).join(BACKUP_DIR);
    fs::create_dir_all(&backup_dir)?;
    Ok(backup_dir.join(BACKUP_FILE))
}

pub fn backup_exists() -> bool {
    get_backup_file_path()
        .map(|path| path.exists())
        .unwrap_or(false)
}

fn read_registry_value(
    hkey: HKEY,
    value_name: &str,
) -> Result<RegistryValue, Box<dyn std::error::Error>> {
    unsafe {
        let value_name_wide = to_wide_string(value_name);
        let mut data_size: u32 = 0;
        let mut value_type = REG_VALUE_TYPE(0);

        RegQueryValueExW(
            hkey,
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            None,
            Some(&mut value_type as *mut REG_VALUE_TYPE),
            None,
            Some(&mut data_size),
        )
        .ok()?;

        let mut data: Vec<u8> = vec![0; data_size as usize];
        RegQueryValueExW(
            hkey,
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            None,
            Some(&mut value_type),
            Some(data.as_mut_ptr()),
            Some(&mut data_size),
        )
        .ok()?;

        data.truncate(data_size as usize);
        Ok(RegistryValue {
            value_type: value_type.0,
            data,
        })
    }
}

fn write_registry_value(
    hkey: HKEY,
    value_name: &str,
    value: &RegistryValue,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let value_name_wide = to_wide_string(value_name);
        RegSetValueExW(
            hkey,
            PCWSTR::from_raw(value_name_wide.as_ptr()),
            Some(0),
            REG_VALUE_TYPE(value.value_type),
            Some(&value.data),
        )
        .ok()?;

        Ok(())
    }
}

fn read_registry_key(hkey: HKEY) -> Result<RegistryKey, Box<dyn std::error::Error>> {
    let mut reg_key = RegistryKey {
        values: HashMap::new(),
        subkeys: HashMap::new(),
    };

    unsafe {
        let mut value_index = 0;
        loop {
            let mut value_name_buf: Vec<u16> = vec![0; 16384];
            let mut value_name_len = value_name_buf.len() as u32;

            let result = RegEnumValueW(
                hkey,
                value_index,
                Some(windows_core::PWSTR::from_raw(value_name_buf.as_mut_ptr())),
                &mut value_name_len,
                None,
                None,
                None,
                None,
            );

            if result.is_err() {
                break;
            }

            value_name_buf.truncate(value_name_len as usize);
            let value_name = from_wide_string(&value_name_buf);

            if let Ok(value) = read_registry_value(hkey, &value_name) {
                reg_key.values.insert(value_name, value);
            }

            value_index += 1;
        }

        let mut subkey_index = 0;
        loop {
            let mut subkey_name_buf: Vec<u16> = vec![0; 255];
            let mut subkey_name_len = subkey_name_buf.len() as u32;

            let result = RegEnumKeyExW(
                hkey,
                subkey_index,
                Some(windows_core::PWSTR::from_raw(subkey_name_buf.as_mut_ptr())),
                &mut subkey_name_len,
                None,
                None,
                None,
                None,
            );

            if result.is_err() {
                break;
            }

            subkey_name_buf.truncate(subkey_name_len as usize);
            let subkey_name = from_wide_string(&subkey_name_buf);

            let subkey_path_wide = to_wide_string(&subkey_name);
            let mut subkey_handle = HKEY::default();
            let result = RegOpenKeyExW(
                hkey,
                PCWSTR::from_raw(subkey_path_wide.as_ptr()),
                Some(0),
                KEY_READ | KEY_WOW64_32KEY,
                &mut subkey_handle,
            );

            if result.is_ok() {
                if let Ok(subkey_data) = read_registry_key(subkey_handle) {
                    reg_key.subkeys.insert(subkey_name, subkey_data);
                }
                let _ = RegCloseKey(subkey_handle);
            }

            subkey_index += 1;
        }
    }

    Ok(reg_key)
}

fn write_registry_key(
    parent_hkey: HKEY,
    key_path: &str,
    key_data: &RegistryKey,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let key_path_wide = to_wide_string(key_path);
        let mut hkey = HKEY::default();

        RegCreateKeyExW(
            parent_hkey,
            PCWSTR::from_raw(key_path_wide.as_ptr()),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE | KEY_WOW64_32KEY,
            None,
            &mut hkey,
            None,
        )
        .ok()?;

        for (value_name, value) in &key_data.values {
            write_registry_value(hkey, value_name, value)?;
        }

        for (subkey_name, subkey_data) in &key_data.subkeys {
            write_registry_key(hkey, subkey_name, subkey_data)?;
        }

        Ok(RegCloseKey(hkey).ok()?)
    }
}

pub fn backup_omnirig_registry() -> Result<RegistryBackup, Box<dyn std::error::Error>> {
    let clsid_path = to_wide_string(format!("CLSID\\{}", OMNIRIG_CLSID).as_str());

    unsafe {
        let mut hkey = HKEY::default();
        let result = RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR::from_raw(clsid_path.as_ptr()),
            Some(0),
            KEY_READ | KEY_WOW64_32KEY,
            &mut hkey,
        );

        if !result.is_ok() {
            println!(
                "Original OmniRig not found in registry. Nothing to backup. (Error: {result:?})"
            );
            return Ok(RegistryBackup {
                clsid_key: None,
                original_exe_path: None,
                backup_timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }

        let clsid_key = read_registry_key(hkey)?;

        let original_exe_path = clsid_key
            .subkeys
            .get("LocalServer32")
            .and_then(|k| k.values.get(""))
            .map(|v| from_wide_string(bytemuck::cast_slice(&v.data)));

        RegCloseKey(hkey).ok()?;

        Ok(RegistryBackup {
            clsid_key: Some(clsid_key),
            original_exe_path,
            backup_timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}

pub fn save_backup(backup: &RegistryBackup) -> Result<(), Box<dyn std::error::Error>> {
    let path = get_backup_file_path()?;
    let json = serde_json::to_string_pretty(backup)?;
    fs::write(&path, json)?;
    println!("Registry backup saved to: {}", path.display());
    Ok(())
}

pub fn load_backup() -> Result<RegistryBackup, Box<dyn std::error::Error>> {
    let path = get_backup_file_path()?;
    let json = fs::read_to_string(&path)?;
    let backup: RegistryBackup = serde_json::from_str(&json)?;
    Ok(backup)
}

pub fn restore_omnirig_registry(backup: &RegistryBackup) -> Result<(), Box<dyn std::error::Error>> {
    let clsid_path = format!("CLSID\\{}", OMNIRIG_CLSID);

    unsafe {
        let clsid_path_wide = to_wide_string(&clsid_path);

        let mut hkey = HKEY::default();
        let open_result = RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR::from_raw(clsid_path_wide.as_ptr()),
            Some(0),
            KEY_READ | KEY_WRITE | KEY_WOW64_32KEY,
            &mut hkey,
        );

        if open_result.is_ok() {
            let delete_result = RegDeleteTreeW(hkey, None);
            RegCloseKey(hkey).ok()?;

            let delete_key_result = RegDeleteKeyExW(
                HKEY_CLASSES_ROOT,
                PCWSTR::from_raw(clsid_path_wide.as_ptr()),
                KEY_WOW64_32KEY.0,
                Some(0),
            );

            if !delete_result.is_ok() || !delete_key_result.is_ok() {
                println!("Warning: Could not fully delete existing CLSID key");
            }
        }
    }

    if let Some(clsid_key) = &backup.clsid_key {
        write_registry_key(HKEY_CLASSES_ROOT, &clsid_path, clsid_key)?;
        println!("Registry restored successfully.");
    } else {
        println!("No original registry data to restore (was not installed).");
    }

    Ok(())
}

pub fn delete_backup() -> Result<(), Box<dyn std::error::Error>> {
    let path = get_backup_file_path()?;
    if path.exists() {
        fs::remove_file(&path)?;
        println!("Backup file deleted: {}", path.display());
    }
    Ok(())
}

pub fn register_holyrig_com_component(
    clsid: &GUID,
    exe_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let clsid_path = format!("CLSID\\{{{:?}}}", clsid);
        let clsid_path_wide = to_wide_string(&clsid_path);

        let mut hkey_clsid = HKEY::default();
        RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR::from_raw(clsid_path_wide.as_ptr()),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE | KEY_WOW64_32KEY,
            None,
            &mut hkey_clsid,
            None,
        )
        .ok()?;

        let default_value = to_wide_string("OmniRigX");
        let default_value_bytes: &[u8] = std::slice::from_raw_parts(
            default_value.as_ptr() as *const u8,
            default_value.len() * 2,
        );
        let result = RegSetValueExW(
            hkey_clsid,
            PCWSTR::from_raw([0u16].as_ptr()),
            Some(0),
            REG_SZ,
            Some(default_value_bytes),
        );

        if !result.is_ok() {
            RegCloseKey(hkey_clsid).ok()?;
            result.ok()?;
        }

        let localserver32_path = format!("CLSID\\{{{:?}}}\\LocalServer32", clsid);
        let localserver32_path_wide = to_wide_string(&localserver32_path);

        let mut hkey_localserver = HKEY::default();
        let result = RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR::from_raw(localserver32_path_wide.as_ptr()),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE | KEY_WOW64_32KEY,
            None,
            &mut hkey_localserver,
            None,
        );

        if !result.is_ok() {
            RegCloseKey(hkey_clsid).ok()?;
            result.ok()?;
        }

        let exe_path_wide = to_wide_string(exe_path);
        let exe_path_bytes: &[u8] = std::slice::from_raw_parts(
            exe_path_wide.as_ptr() as *const u8,
            exe_path_wide.len() * 2,
        );
        let result = RegSetValueExW(
            hkey_localserver,
            PCWSTR::from_raw([0u16].as_ptr()),
            Some(0),
            REG_SZ,
            Some(exe_path_bytes),
        );

        if !result.is_ok() {
            RegCloseKey(hkey_localserver).ok()?;
            RegCloseKey(hkey_clsid).ok()?;
            result.ok()?;
        }

        RegCloseKey(hkey_localserver).ok()?;
        RegCloseKey(hkey_clsid).ok()?;
    }

    println!("HolyRig OmniRig registered successfully.");
    Ok(())
}

pub fn get_current_omnirig_path() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let clsid_path = format!("CLSID\\{}\\LocalServer32", OMNIRIG_CLSID);
    let clsid_path_wide = to_wide_string(&clsid_path);

    unsafe {
        let mut hkey = HKEY::default();
        let result = RegOpenKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR::from_raw(clsid_path_wide.as_ptr()),
            Some(0),
            KEY_READ | KEY_WOW64_32KEY,
            &mut hkey,
        );

        if !result.is_ok() {
            Ok(None)
        } else {
            let value = read_registry_value(hkey, "")?;
            RegCloseKey(hkey).ok()?;

            let path = from_wide_string(bytemuck::cast_slice(&value.data));
            Ok(Some(path))
        }
    }
}
