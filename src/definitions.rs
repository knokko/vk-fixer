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
