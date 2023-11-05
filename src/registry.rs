use crate::definitions::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::env::var;
use std::fs::File;
use std::io::BufReader;
use std::process::Command;
use windows::Win32::System::Registry::*;
use windows::core::*;

#[derive(Debug, Deserialize)]
struct RootLayerSettings {
    layer: Option<LayerSettings>,
    layers: Option<Vec<LayerSettings>>
}

#[derive(Debug, Deserialize)]
struct LayerSettings {
    name: String,
    description: String,
    disable_environment: HashMap<String, String>,
    enable_environment: Option<HashMap<String, String>>
}

pub fn get_implicit_layers() -> (Vec<ImplicitLayer>, Vec<String>) {
    let mut errors = Vec::new();
    let user_layers = enumerate_layers_of_hkey(
        HKEY_CURRENT_USER, ImplicitRegistry::CurrentUser, &mut errors
    );
    let machine_layers = enumerate_layers_of_hkey(
        HKEY_LOCAL_MACHINE, ImplicitRegistry::LocalMachine, &mut errors
    );

    if let Err(user_error) = &user_layers {
        errors.push(user_error.message().to_string());
    }
    if let Err(machine_error) = &machine_layers {
        errors.push(machine_error.message().to_string());
    }

    ([user_layers.unwrap_or(vec![]), machine_layers.unwrap_or(vec![])].concat(), errors)
}

fn enumerate_keys_of_hkey(root_hkey: HKEY, path: PCSTR) -> Result<Vec<String>> {
    let mut hkey = HKEY::default();
    let mut num_keys = 0;
    let mut longest_key_length = 0;

    unsafe {
        RegOpenKeyExA(
            root_hkey, path, 0, KEY_READ, &mut hkey
        )?;
        RegQueryInfoKeyA(
            hkey, PSTR::null(), None, None, None,
            None, None, Some(&mut num_keys),
            Some(&mut longest_key_length),
            None, None, None
        )?;

        let mut result = Vec::with_capacity(num_keys as usize);

        let mut key_holder = String::with_capacity(longest_key_length as usize);
        let p_key = PSTR(key_holder.as_mut_ptr());

        for index in 0 ..num_keys {
            let mut current_key_length = longest_key_length + 1;
            RegEnumValueA(
                hkey, index, p_key,
                &mut current_key_length, None,
                None, None, None
            )?;

            result.push(p_key.to_string()?);
        }
        Ok(result)
    }
}

fn enumerate_layers_of_hkey(root_hkey: HKEY, registry: ImplicitRegistry, errors: &mut Vec<String>) -> Result<Vec<ImplicitLayer>> {
    let layer_paths = enumerate_keys_of_hkey(
        root_hkey, s!("SOFTWARE\\Khronos\\Vulkan\\ImplicitLayers")
    )?;

    let mut result = Vec::with_capacity(layer_paths.len());
    for settings_path in layer_paths {
        extract_layer_settings(&settings_path, registry, &mut result, errors);
    }

    Ok(result)
}

fn extract_layer_settings(path: &str, registry: ImplicitRegistry, dest: &mut Vec<ImplicitLayer>, errors: &mut Vec<String>) {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        let layer_settings: serde_json::error::Result<RootLayerSettings> = serde_json::from_reader(reader);

        match layer_settings {
            Err(parse_error) => errors.push(format!("Failed to parse {}: {}", path, parse_error)),
            Ok(settings) => {
                if let Some(layer) = settings.layer {
                    extract_single_layer_settings(layer, path, registry, dest, errors);
                }
                if let Some(layers) = settings.layers {
                    for layer in layers {
                        extract_single_layer_settings(layer, path, registry, dest, errors);
                    }
                }
            }
        };
    } else {
        errors.push(format!("Failed to open file {}", path));
    }
}

fn extract_single_layer_settings(layer: LayerSettings, settings_path: &str, registry: ImplicitRegistry, dest: &mut Vec<ImplicitLayer>, errors: &mut Vec<String>) {
    let get_disable_environment = layer.disable_environment.keys().into_iter().next();
    if let Some(disable_environment) = get_disable_environment {
        let enable_environment = match layer.enable_environment {
            Some(environment_map) => environment_map.keys().into_iter().next().map(|key| key.clone()),
            None => None
        };
        dest.push(ImplicitLayer {
            settings_path: settings_path.to_string(),
            registry,
            name: layer.name,
            description: layer.description,
            disable_environment: disable_environment.clone(),
            enable_environment
        });
    } else {
        errors.push(format!("Layer {} has empty disable_environment", settings_path));
    }
}

fn spawn_remove_command(key: &str) -> bool {
    Command::new("reg").args(["delete", "HKCU\\Environment", "/v", key, "/f"]).spawn().is_ok()
}
pub fn remove_user_environment(key: &str) -> bool {
    if spawn_remove_command(key) {

        // Unlike the setx command, the reg delete command will NOT refresh the environment
        // variables. By setting and unsetting a dummy variable, I abuse setx the refresh the
        // environment variable I care about.
        let dummy = "force-refresh-dummy";
        set_user_environment(dummy);
        let _ = spawn_remove_command(dummy);
        true
    } else { false }
}

pub fn set_user_environment(key: &str) -> bool {
    Command::new("setx").args([key, "1"]).spawn().is_ok()
}

pub fn get_global_environment_keys() -> EnvironmentVariables {
    let mut errors = Vec::new();
    let user_keys = enumerate_keys_of_hkey(
        HKEY_CURRENT_USER, s!("Environment")
    );
    let system_keys = enumerate_keys_of_hkey(
        HKEY_LOCAL_MACHINE,
        s!("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
    );

    if let Err(user_error) = &user_keys {
        errors.push(user_error.message().to_string());
    }
    if let Err(system_error) = &system_keys {
        errors.push(system_error.message().to_string());
    }

    EnvironmentVariables {
        user: user_keys.unwrap_or(vec![]),
        system: system_keys.unwrap_or(vec![]),
        errors,
    }
}

pub fn is_enabled(layer: &ImplicitLayer) -> bool {
    if var(&layer.disable_environment).is_ok() {
        return false;
    }
    if let Some(enable_env) = &layer.enable_environment {
        return var(enable_env).is_ok();
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parsing_single_layer() {
        let mut layers = Vec::new();
        let mut errors = Vec::new();
        extract_layer_settings(
            "./single-layer-manifest.json",
            ImplicitRegistry::LocalMachine,
            &mut layers, &mut errors
        );
        assert_eq!(Vec::<String>::new(), errors);
        assert_eq!(vec![
            ImplicitLayer {
                settings_path: "./single-layer-manifest.json".to_string(),
                registry: ImplicitRegistry::LocalMachine,
                name: "VK_LAYER_LUNARG_overlay".to_string(),
                description: "LunarG HUD layer".to_string(),
                disable_environment: "DISABLE_LAYER_OVERLAY_1".to_string(),
                enable_environment: Some("ENABLE_LAYER_OVERLAY_1".to_string())
            }
        ], layers);
    }

    #[test]
    fn test_parsing_multiple_layers() {
        let mut layers = Vec::new();
        let mut errors = Vec::new();
        extract_layer_settings(
            "./multiple-layers-manifest.json",
            ImplicitRegistry::CurrentUser,
            &mut layers, &mut errors
        );
        assert_eq!(Vec::<String>::new(), errors);
        assert_eq!(vec![
            ImplicitLayer {
                settings_path: "./multiple-layers-manifest.json".to_string(),
                registry: ImplicitRegistry::CurrentUser,
                name: "VK_LAYER_LUNARG_overlay - multiple".to_string(),
                description: "LunarG HUD layer".to_string(),
                disable_environment: "DISABLE_LAYER_OVERLAY_1".to_string(),
                enable_environment: None
            }
        ], layers);
    }
}
