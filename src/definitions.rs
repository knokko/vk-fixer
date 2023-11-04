#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ImplicitRegistry {
    CurrentUser, LocalMachine
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ImplicitLayer {
    pub settings_path: String,
    pub registry: ImplicitRegistry,
    pub name: String,
    pub description: String,
    pub disable_environment: String,
    pub enable_environment: Option<String>
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TrialResult {
    pub exit_code: i32,
    pub output: String
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct VersionedTrialResults {
    pub vk10: TrialResult,
    pub vk11: TrialResult,
    pub vk12: TrialResult,
    pub vk13: TrialResult
}

impl VersionedTrialResults {
    pub fn all_succeeded(&self) -> bool {
        [&self.vk10, &self.vk11, &self.vk12, &self.vk13].into_iter().all(|trial| trial.exit_code == 0)
    }

    pub fn all_failed(&self) -> bool {
        [&self.vk10, &self.vk11, &self.vk12, &self.vk13].into_iter().all(|trial| trial.exit_code != 0)
    }

    pub fn succeeded(&self, api_version: u32) -> bool {
        match api_version {
            ash::vk::API_VERSION_1_0 => self.vk10.exit_code == 0,
            ash::vk::API_VERSION_1_1 => self.vk11.exit_code == 0,
            ash::vk::API_VERSION_1_2 => self.vk12.exit_code == 0,
            ash::vk::API_VERSION_1_3 => self.vk13.exit_code == 0,
            _ => panic!("Unexpected api version {}", api_version)
        }
    }

    pub fn succeeded_except(&self, ignored_api_versions: &[u32]) -> bool {
        if self.vk10.exit_code != 0 && !ignored_api_versions.contains(&ash::vk::API_VERSION_1_0) {
            return false;
        }
        if self.vk11.exit_code != 0 && !ignored_api_versions.contains(&ash::vk::API_VERSION_1_1) {
            return false;
        }
        if self.vk12.exit_code != 0 && !ignored_api_versions.contains(&ash::vk::API_VERSION_1_2) {
            return false;
        }
        if self.vk13.exit_code != 0 && !ignored_api_versions.contains(&ash::vk::API_VERSION_1_3) {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct TestResults {
    /// The result of running the test app without disabling any implicit layers
    pub default_result: VersionedTrialResults,
    /// The result of running the test app while disabling all implicit layers
    pub clean_result: VersionedTrialResults,
    /// The results of running the test app, where a different implicit layer is blocked in
    /// each trial
    pub exclude_results: Vec<(String, VersionedTrialResults)>,
    /// The result of running the test app, where all implicit layers are blocked, except 1 in
    /// each trial
    pub isolation_results: Vec<(String, VersionedTrialResults)>
}

pub struct EnvironmentVariables {
    pub user: Vec<String>,
    pub system: Vec<String>,
    pub errors: Vec<String>
}

#[derive(PartialEq, Eq, Debug)]
pub enum Conclusion {
    /// All trials succeeded, so the implicit layers are probably fine.
    Healthy,
    /// The default trial (with all implicit layers) succeeded, but not all trials succeeded.
    WeirdHealthy,
    /// All trials failed: the machine appears unable to run any Vulkan application,
    /// but it doesn't seem to be caused by implicit layers.
    Hopeless,
    /// Both the default trial (with all layers) and the clean trial (without any layers) failed,
    /// but not all trials failed.
    WeirdBroken { important_layer: String, exclude: bool },
    /// All trials for some Vulkan version(s) failed, but all others succeeded.
    /// This probably means that the graphics drivers don't support later versions.
    Partial { supported_versions: Vec<u32> },
    /// One of the implicit layers appears to be completely broken (even when all other layers
    /// are disabled).
    BrokenLayer { layer: String },
    /// It looks like one of the implicit layers only supports a subset of the Vulkan versions
    /// supported by the graphics drivers.
    PartiallyBrokenLayer { layer: String, broken_versions: Vec<u32> },
    /// Multiple layers are conflicting: the trials succeed when at least 1 of them is disabled.
    SymmetricConflict { layers: Vec<String> },
    /// One layer conflicts with multiple other layers: all trials where both `main_offender` and
    /// another layer were enabled failed. All trials without `main_offender` and with only
    /// `main_offender` succeeded.
    AsymmetricConflict { main_offender: String },
    /// Multiple layers are conflicting with each other. All layers work fine in isolation, but
    /// all trials with more than 1 active layer failed.
    ComplexConflict,
}
