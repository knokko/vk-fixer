use registry::get_implicit_layers;
use std::process::Command;

fn main() {
    let (implicit_layers, errors) = get_implicit_layers();
    if errors.len() > 0 {
        println!("Errors are:");
        for error in errors {
            println!("  {}", error);
        }
        println!();
    }

    println!("Implicit layers are:");
    for layer in implicit_layers {
        println!("  {:?}", layer);

        if layer.name == Some("VK_LAYER_OBS_HOOK".to_string()) {
            let (disable_key, disable_value) = layer.disable_environment.unwrap();
            println!("{:?}", Command::new("vulkaninfo").args(["--summary"]).env(disable_key, disable_value).output());
        }
    }
}
