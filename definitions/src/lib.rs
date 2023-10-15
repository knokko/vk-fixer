#[derive(Debug, Clone, Copy)]
pub enum ImplicitRegistry {
    CurrentUser, LocalMachine
}

#[derive(Debug, Clone)]
pub struct ImplicitLayer {
    pub settings_path: String,
    pub registry: ImplicitRegistry,
    pub name: Option<String>,
    pub disable_environment: Option<(String, String)>
}
