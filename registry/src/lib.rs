use definitions::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, ErrorKind};
use windows::Win32::System::Registry::*;
use windows::core::*;

#[derive(Debug, Deserialize)]
struct RootLayerSettings {
    layer: LayerSettings
}

#[derive(Debug, Deserialize)]
struct LayerSettings {
    name: String,
    disable_environment: HashMap<String, String>
}

pub fn get_implicit_layers() -> (Vec<ImplicitLayer>, Vec<String>) {
    let user_layers = enumerate_layers_of_hkey(HKEY_CURRENT_USER, ImplicitRegistry::CurrentUser);
    let machine_layers = enumerate_layers_of_hkey(HKEY_LOCAL_MACHINE, ImplicitRegistry::LocalMachine);

    let mut errors = Vec::new();
    if let Err(user_error) = &user_layers {
        errors.push(user_error.message().to_string());
    }
    if let Err(machine_error) = &machine_layers {
        errors.push(machine_error.message().to_string());
    }

    ([user_layers.unwrap_or(vec![]), machine_layers.unwrap_or(vec![])].concat(), errors)
}

fn enumerate_layers_of_hkey(root_hkey: HKEY, registry: ImplicitRegistry) -> Result<Vec<ImplicitLayer>> {
    let mut hkey = HKEY::default();
    let mut num_layers = 0;
    let mut longest_layer_name_length = 0;

    unsafe {
        RegOpenKeyExA(
            root_hkey, s!("SOFTWARE\\Khronos\\Vulkan\\ImplicitLayers"),
            0, KEY_READ, &mut hkey
        )?;
        RegQueryInfoKeyA(
            hkey, PSTR::null(), None, None, None,
            None, None, Some(&mut num_layers),
            Some(&mut longest_layer_name_length),
            None, None, None
        )?;

        let mut result = Vec::with_capacity(num_layers as usize);

        let mut layer_name_holder = String::with_capacity(longest_layer_name_length as usize);
        let p_layer_name = PSTR(layer_name_holder.as_mut_ptr());

        for index in 0 .. num_layers {
            let mut current_layer_name_length = longest_layer_name_length + 1;
            RegEnumValueA(
                hkey, index, p_layer_name,
                &mut current_layer_name_length, None,
                None, None, None
            )?;

            let settings_path = p_layer_name.to_string()?;
            let try_layer_settings = extract_layer_settings(&settings_path);
            if let Ok((name, disable_key, disable_value)) = try_layer_settings {
                result.push(ImplicitLayer {
                    settings_path,
                    registry,
                    name: Some(name),
                    disable_environment: Some((disable_key, disable_value))
                });
            } else {
                result.push(ImplicitLayer {
                    settings_path,
                    registry,
                    name: None,
                    disable_environment: None
                })
            }
        }
        Ok(result)
    }
}

fn extract_layer_settings(path: &str) -> std::io::Result<(String, String, String)> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let layer_settings: RootLayerSettings = serde_json::from_reader(reader)?;

    let disable_map = &layer_settings.layer.disable_environment;

    for key in disable_map.keys() {
        return Ok((layer_settings.layer.name, key.clone(), disable_map[key].clone()));
    }

    Err(std::io::Error::new(ErrorKind::InvalidData, "disable_environment is empty"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        println!("{:?}", enumerate_layers_of_hkey(HKEY_LOCAL_MACHINE, ImplicitRegistry::LocalMachine));
        println!("{:?}", enumerate_layers_of_hkey(HKEY_CURRENT_USER, ImplicitRegistry::CurrentUser));
    }
}
