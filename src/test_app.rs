use ash::vk;
use std::collections::HashMap;
use std::default::Default;
use std::env::args;
use std::process::{exit, Command, Child, Stdio};
use std::str::FromStr;
use crate::definitions::{TrialResult, VersionedTrialResults};

pub fn maybe_run_test_app() {
    let args = args().collect::<Vec<_>>();
    if args.len() == 3 && args[1] == "test-app" {
        if let Ok(api_version) = u32::from_str(&args[2]) {
            run_test_app(api_version);
            exit(0);
        }
    }
}

pub fn await_test_apps(children: [std::io::Result<Child>; 4]) -> VersionedTrialResults {
    let [vk10, vk11, vk12, vk13] = children;
    VersionedTrialResults {
        vk10: await_test_app(vk10),
        vk11: await_test_app(vk11),
        vk12: await_test_app(vk12),
        vk13: await_test_app(vk13),
    }
}

fn await_test_app(child: std::io::Result<Child>) -> TrialResult {
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

pub fn spawn_test_apps(envs: &[&str]) -> [std::io::Result<Child>; 4] {
    [
        spawn_test_app(envs, vk::API_VERSION_1_0),
        spawn_test_app(envs, vk::API_VERSION_1_1),
        spawn_test_app(envs, vk::API_VERSION_1_2),
        spawn_test_app(envs, vk::API_VERSION_1_3),
    ]
}

fn spawn_test_app(envs: &[&str], api_version: u32) -> std::io::Result<Child> {
    let mut env_map: HashMap<&str, &str> = HashMap::new();
    for key in envs {
        env_map.insert(key, "1");
    }

    Command::new(
        args().next().expect("First arg should be path to own exe file")
    ).args(["test-app", &api_version.to_string()]).stdout(Stdio::piped()).stderr(Stdio::piped())
        .envs(env_map).spawn()
}

fn run_test_app(api_version: u32) {
    unsafe {
        let raw_entry = ash::Entry::load();
        if let Err(entry_error) = raw_entry {
            print!("Failed to load Entry: {:?}", entry_error);
            exit(-21023);
        }
        let entry = raw_entry.unwrap();

        let mut app_info = vk::ApplicationInfo::default();
        app_info.api_version = api_version;

        let mut ci_instance = vk::InstanceCreateInfo::default();
        ci_instance.p_application_info = &app_info;

        let raw_instance = entry
            .create_instance(&ci_instance, None);
        if let Err(instance_error) = raw_instance {
            print!("Failed to create VkInstance: {:?}", instance_error);
            exit(-21024);
        }
        let instance = raw_instance.unwrap();

        let raw_physical_device = instance
            .enumerate_physical_devices();
        if let Err(device_error) = raw_physical_device {
            print!("Failed to enumerate physical devices: {:?}", device_error);
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
            print!("Failed to create VkDevice: {:?}", device_error);
            exit(-21026);
        }
        let device = raw_device.unwrap();

        device.destroy_device(None);
        instance.destroy_instance(None);
    }
}
