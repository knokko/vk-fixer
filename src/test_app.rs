use ash::vk;
use std::collections::HashMap;
use std::default::Default;
use std::env::args;
use std::process::{exit, Command, Child};
use crate::definitions::TrialResult;

pub fn maybe_run_test_app() {
    let args = args().collect::<Vec<_>>();
    if args.len() == 2 && args[1] == "test-app" {
        run_test_app();
        exit(0);
    }
}

pub fn await_test_app(child: std::io::Result<Child>) -> TrialResult {
    match child.map(|t| t.wait_with_output()) {
        Err(weird) => TrialResult {
            exit_code: -21021,
            output: format!("Failed to launch: {:?}", weird)
        },
        Ok(child_result) => {
            match child_result {
                Ok(result) => {
                    let mut output = String::new();
                    if result.stderr.is_empty() && !result.stdout.is_empty() {
                        output = String::from_utf8(result.stdout.clone()).unwrap_or("Invalid stdout".to_string());
                    }
                    if !result.stderr.is_empty() && result.stdout.is_empty() {
                        output = String::from_utf8(result.stderr.clone()).unwrap_or("Invalid stderr".to_string());
                    }
                    if !result.stderr.is_empty() && !result.stdout.is_empty() {
                        output = String::from_utf8(result.stdout.clone()).unwrap_or("Invalid stdout".to_string()) +
                            "stderr: " + &String::from_utf8(result.stderr.clone()).unwrap_or("Invalid stderr".to_string());
                    }
                    TrialResult {
                        exit_code: result.status.code().unwrap_or(-21022),
                        output
                    }
                },
                Err(no_result) => TrialResult {
                    exit_code: -21020,
                    output: format!("Failed to get result: {:?}", no_result)
                }
            }
        }
    }

}

pub fn spawn_test_app(envs: &[&str]) -> std::io::Result<Child> {
    let mut env_map: HashMap<&str, &str> = HashMap::new();
    for key in envs {
        env_map.insert(key, "1");
    }

    Command::new(
        args().next().expect("First arg should be path to own exe file")
    ).arg("test-app").envs(env_map).spawn()
}

fn run_test_app() {
    unsafe {
        let raw_entry = ash::Entry::load();
        if let Err(entry_error) = raw_entry {
            println!("Failed to load Entry: {:?}", entry_error);
            exit(-21023);
        }
        let entry = raw_entry.unwrap();

        let ci_instance = vk::InstanceCreateInfo::default();

        let raw_instance = entry
            .create_instance(&ci_instance, None);
        if let Err(instance_error) = raw_instance {
            println!("Failed to create VkInstance: {:?}", instance_error);
            exit(-21024);
        }
        let instance = raw_instance.unwrap();

        let raw_physical_device = instance
            .enumerate_physical_devices();
        if let Err(device_error) = raw_physical_device {
            println!("Failed to enumerate physical devices: {:?}", device_error);
            exit(-21025);
        }
        let physical_device = raw_physical_device.unwrap()[0];

        let queue_priorities = 1.0;

        let mut queue_info = vk::DeviceQueueCreateInfo::default();
        queue_info.queue_family_index = 0; // TODO Test that we pick the right one
        queue_info.queue_count = 1;
        queue_info.p_queue_priorities = &queue_priorities;

        let mut ci_device = vk::DeviceCreateInfo::default();
        ci_device.p_queue_create_infos = &queue_info;
        ci_device.queue_create_info_count = 1;

        let raw_device = instance
            .create_device(physical_device, &ci_device, None);
        if let Err(device_error) = raw_device {
            println!("Failed to create VkDevice: {:?}", device_error);
            exit(-21026);
        }
        let device = raw_device.unwrap();

        device.destroy_device(None);
        instance.destroy_instance(None);
    }
}
