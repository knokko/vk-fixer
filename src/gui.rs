use std::cell::RefCell;
use std::io::ErrorKind;
use std::rc::Rc;
use crate::definitions::*;
use crate::logic::{draw_conclusion, run_all_trials};
use crate::registry::*;

#[derive(Eq, PartialEq, Clone, Default)]
pub enum GuiState {
    #[default]
    Initial,
    Manual(bool),
    AutoLayerList,
    AutoResultsTable(TestResults, Vec<ImplicitLayer>),
    AutoConclusion(Conclusion, Vec<ImplicitLayer>, bool),
    AutoFinished(bool),
    AutoFailed(String, bool),
    Exit
}

#[derive(Default, nwd::NwgUi)]
pub struct FixerApp {
    #[nwg_control(size: (750, 315), center: true, title: "vk-fixer", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [FixerApp::say_goodbye] )]
    pub window: nwg::Window,

    #[nwg_control(text: "Automatic mode", size: (300, 50), position: (50, 200))]
    #[nwg_events( OnButtonClick: [FixerApp::start_automatic_mode] )]
    pub automatic_button: nwg::Button,

    #[nwg_control(text: "Manual mode", size: (300, 50), position: (400, 200))]
    #[nwg_events( OnButtonClick: [FixerApp::start_manual_mode] )]
    pub manual_button: nwg::Button,

    pub state: Rc<RefCell<GuiState>>
}

impl FixerApp {

    fn say_goodbye(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }

    fn start_automatic_mode(&self) {
        *self.state.borrow_mut() = GuiState::AutoLayerList;
        nwg::stop_thread_dispatch();
    }

    fn start_manual_mode(&self) {
        *self.state.borrow_mut() = GuiState::Manual(true);
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, nwd::NwgUi)]
pub struct ManualApp {
    #[nwg_events( OnWindowClose: [ManualApp::close], OnInit: [ManualApp::init_layers] )]
    #[nwg_control(size: (750, 700), center: true, title: "Manual layer selection", flags: "MAIN_WINDOW|VISIBLE")]
    pub window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 0, margin: [0, 20, 0, 20])]
    pub layout: nwg::GridLayout,

    pub layer_names: Rc<RefCell<Vec<nwg::CheckBox>>>,
    pub layer_info: RefCell<Vec<nwg::Label>>,
    pub break_buttons: RefCell<Vec<nwg::Button>>,
    pub handlers: RefCell<Vec<nwg::EventHandler>>,

    pub state: Rc<RefCell<GuiState>>,
    pub show_break_buttons: bool,
}

impl ManualApp {
    fn init_layers(&self) {
        let (mut layers, errors) = get_implicit_layers();
        let env = get_global_environment_keys();

        layers.sort_by_key(|layer| {
            if layer.enable_environment.is_some() { 1 } else { 0 }
        });

        let add_info = |text: &str| {
            let mut label = Default::default();
            nwg::Label::builder()
                .text(text)
                .parent(&self.window)
                .build(&mut label)
                .expect("Failed to add info");
            let mut layer_info = self.layer_info.borrow_mut();
            self.layout.add_child_item(nwg::GridLayoutItem::new(
                &label,
                0,
                (self.layer_names.borrow().len() + layer_info.len()) as u32,
                7,
                1
            ));
            layer_info.push(label);
        };

        add_info("Disable layers by checking the corresponding boxes.");
        add_info("Note: depending on the game and how the game is launched,");
        add_info("a computer restart may or may not be needed.");
        add_info("");
        add_info("Alternatively, you can Break layers, which will stop them instantly.");
        if self.show_break_buttons {
            add_info("However, breaking usually requires administrator privileges.");
            add_info("Also, repairing a broken layer is hard, so only do this when you don't need it anymore.");
        } else {
            add_info("If you want to Break layers, you need to restart this application");
            add_info("with administrator privileges.");
        }
        add_info("");
        for layer in layers {

            let mut break_button = Default::default();
            nwg::Button::builder()
                .text("Break")
                .parent(&self.window)
                .build(&mut break_button)
                .expect("Failed to add break button");
            let break_button_handle = break_button.handle;

            let mut layer_box = Default::default();
            let is_disabled = env.user.contains(&layer.disable_environment);
            nwg::CheckBox::builder()
                .text(&layer.name)
                .check_state(if is_disabled { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked })
                .parent(&self.window)
                .build(&mut layer_box)
                .expect("Failed to add layer checkbox");

            let layer_path = layer.settings_path.clone();
            let state_ref = Rc::clone(&self.state);
            let break_handler = nwg::bind_event_handler(
                &break_button.handle, &self.window.handle, move |evt, _evt_data, handle| {
                    if evt == nwg::Event::OnButtonClick && handle == break_button_handle {
                        let delete_result = std::fs::remove_file(&layer_path);
                        if let Err(failed_delete) = delete_result {
                            if failed_delete.kind() == ErrorKind::PermissionDenied {
                                *state_ref.borrow_mut() = GuiState::Manual(false);
                            }
                        }

                        nwg::stop_thread_dispatch();
                    }
                }
            );

            let toggle_handle = layer_box.handle;
            let disable_env = layer.disable_environment.clone();
            let layer_names_ref = Rc::clone(&self.layer_names);
            let toggle_handler = nwg::bind_event_handler(
                &layer_box.handle, &self.window.handle, move |evt, _evt_data, handle| {
                    if evt == nwg::Event::OnButtonClick && handle == toggle_handle {
                        let mut is_disabled = get_global_environment_keys().user.contains(&disable_env);
                        if is_disabled {
                            is_disabled = !remove_user_environment(&disable_env);
                        } else {
                            is_disabled = set_user_environment(&disable_env);
                        }

                        let layer_boxes = layer_names_ref.borrow_mut();
                        for layer_box in &*layer_boxes {
                            if layer_box.handle == toggle_handle {
                                layer_box.set_check_state(if is_disabled { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked });
                            }
                        }
                    }
                });
            self.handlers.borrow_mut().push(break_handler);
            self.handlers.borrow_mut().push(toggle_handler);

            let mut layer_names = self.layer_names.borrow_mut();
            self.layout.add_child_item(nwg::GridLayoutItem::new(
                &layer_box,
                0,
                (layer_names.len() + self.layer_info.borrow().len()) as u32,
                7, 1
            ));
            layer_names.push(layer_box);
            drop(layer_names);

            if self.show_break_buttons {
                let mut break_buttons = self.break_buttons.borrow_mut();
                self.layout.add_child_item(nwg::GridLayoutItem::new(
                    &break_button,
                    7,
                    (self.layer_names.borrow().len() + self.layer_info.borrow().len() - 1) as u32,
                    1, 1
                ));
                break_buttons.push(break_button);
                drop(break_buttons);
            }

            add_info(&layer.description);
            if let Some(enable_env) = layer.enable_environment {
                if !env.user.contains(&enable_env) && !env.system.contains(&enable_env) {
                    add_info("Note: this layer is NOT enabled by default, so disabling it has probably no effect");
                }
            }
            if env.system.contains(&layer.disable_environment) {
                add_info("Note: this layer is already disabled system-wide, so disabling it has probably no effect");
            }
            add_info("");
        }

        if errors.len() > 0 {
            add_info("Some errors occurred while enumerating layers:");
            for error in errors {
                add_info(&error);
            }
        }
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, nwd::NwgUi)]
pub struct AutoLayerApp {
    #[nwg_events( OnWindowClose: [AutoLayerApp::close], OnInit: [AutoLayerApp::init_layers] )]
    #[nwg_control(size: (650, 400), center: true, title: "Automatic: layer list", flags: "MAIN_WINDOW|VISIBLE")]
    pub window: nwg::Window,

    #[nwg_control(text: "Run trials...", size: (300, 50), position: (150, 325))]
    #[nwg_events( OnButtonClick: [AutoLayerApp::run_trials] )]
    pub trials_button: nwg::Button,

    #[nwg_layout(parent: window, spacing: 0, margin: [0, 50, 100, 50])]
    pub layout: nwg::GridLayout,

    pub layer_info: RefCell<Vec<nwg::Label>>,

    pub layer_list: RefCell<Vec<ImplicitLayer>>,

    pub state: Rc<RefCell<GuiState>>
}

impl AutoLayerApp {
    fn init_layers(&self) {
        let (mut layers, errors) = get_implicit_layers();

        layers.retain(|layer| is_enabled(layer));

        let add_info = |text: &str| {
            let mut label = Default::default();
            nwg::Label::builder()
                .text(text)
                .parent(&self.window)
                .build(&mut label)
                .expect("Failed to add info");

            let mut layer_info = self.layer_info.borrow_mut();
            self.layout.add_child(0, layer_info.len() as u32, &label);
            layer_info.push(label);
        };

        if layers.is_empty() {
            add_info("No truly implicit layers were found on your system.");
            add_info("If you can't run any Vulkan game, you may have bad graphics drivers.");
            add_info("You can still run the trials, but they probably won't help.");
        } else {
            add_info("The following implicit layers will be tested:");
        }
        for layer in &layers {
            add_info("");
            add_info(&format!("Name: {}", &layer.name));
            add_info(&format!("Description: {}", &layer.description));
        }

        if errors.len() > 0 {
            add_info("");
            add_info("Some errors occurred while enumerating layers:");
            for error in errors {
                add_info(&error);
            }
        }

        add_info("");
        add_info("Note: running all trials can take several seconds.");

        *self.layer_list.borrow_mut() = layers;
    }

    fn run_trials(&self) {
        let layers = self.layer_list.borrow().clone();
        let results = run_all_trials(&layers);
        *self.state.borrow_mut() = GuiState::AutoResultsTable(results, layers);
        nwg::stop_thread_dispatch();
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, nwd::NwgUi)]
pub struct AutoResultsApp {
    #[nwg_events( OnWindowClose: [AutoResultsApp::close], OnInit: [AutoResultsApp::init_results_table] )]
    #[nwg_control(size: (1400, 700), center: true, title: "Automatic: trial results", flags: "MAIN_WINDOW|VISIBLE")]
    pub window: nwg::Window,

    #[nwg_control(text: "Jump to conclusions", size: (300, 50), position: (150, 125))]
    #[nwg_events( OnButtonClick: [AutoResultsApp::jump_to_conclusions] )]
    pub conclusions_button: nwg::Button,

    #[nwg_layout(parent: window, spacing: 0, margin: [0, 50, 600, 50])]
    pub info_layout: nwg::GridLayout,

    #[nwg_layout(parent: window, spacing: 0, margin: [200, 0, 0, 0])]
    pub table_layout: nwg::GridLayout,

    pub results: TestResults,
    pub layers: Vec<ImplicitLayer>,

    pub results_table: RefCell<Vec<nwg::Label>>,
    pub info_labels: RefCell<Vec<nwg::Label>>,

    pub state: Rc<RefCell<GuiState>>
}

impl AutoResultsApp {
    fn init_results_table(&self) {
        let add_info = |text: &str| {
            let mut label = Default::default();
            nwg::Label::builder()
                .text(text)
                .parent(&self.window)
                .build(&mut label)
                .expect("Failed to add line");

            let mut line = self.info_labels.borrow_mut();
            self.info_layout.add_child(0, line.len() as u32, &label);
            line.push(label);
        };

        let add_entry = |description: &str, vk10: &str, vk11: &str, vk12: &str, vk13: &str| {
            let columns = [description, vk10, vk11, vk12, vk13];
            for index in 0 .. columns.len() {
                let column = columns[index];
                let mut label = Default::default();
                nwg::Label::builder()
                    .text(column)
                    .parent(&self.window)
                    .build(&mut label)
                    .expect("Failed to add entry");

                let mut entries = self.results_table.borrow_mut();
                self.table_layout.add_child(index as u32, entries.len() as u32 / 5, &label);
                entries.push(label);
            }
        };

        let add_results_entry = |description: &str, results: &VersionedTrialResults| {
            add_entry(
                description,
                &results.vk10.exit_code.to_string(),
                &results.vk11.exit_code.to_string(),
                &results.vk12.exit_code.to_string(),
                &results.vk13.exit_code.to_string()
            );
        };

        add_info("The raw trial results are shown in the 'table' below.");
        add_info("0 indicates a success; everything else indicates a failure.");
        add_info("If you don't (want to) understand it, just click on 'Jump to conclusions',");
        add_info("which will show the most likely issues, and offer to fix them.");

        add_entry("trial description", "vk1.0 result", "vk1.1 result", "vk1.2 result", "vk1.3 result");
        add_results_entry("with all layers", &self.results.default_result);
        add_results_entry("without any layers", &self.results.clean_result);
        for (layer, results) in &self.results.exclude_results {
            add_results_entry(&format!("without {}", layer), results);
        }
        for (layer, results) in &self.results.isolation_results {
            add_results_entry(&format!("only {}", layer), results);
        }
    }

    fn jump_to_conclusions(&self) {
        let conclusion = draw_conclusion(&self.results);
        *self.state.borrow_mut() = GuiState::AutoConclusion(conclusion, self.layers.clone(), true);
        nwg::stop_thread_dispatch();
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, nwd::NwgUi)]
pub struct AutoConclusionApp {
    #[nwg_events( OnWindowClose: [AutoConclusionApp::close], OnInit: [AutoConclusionApp::init_conclusion] )]
    #[nwg_control(size: (900, 400), center: true, title: "Automatic: conclusion", flags: "MAIN_WINDOW|VISIBLE")]
    pub window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 0, margin: [20, 20, 20, 20])]
    pub table_layout: nwg::GridLayout,

    pub conclusion: Conclusion,

    pub layers: Vec<ImplicitLayer>,

    pub show_break_buttons: bool,

    pub lines: RefCell<Vec<nwg::Label>>,
    pub buttons: RefCell<Vec<nwg::Button>>,
    pub handlers: RefCell<Vec<nwg::EventHandler>>,

    pub state: Rc<RefCell<GuiState>>
}

impl AutoConclusionApp {
    fn init_conclusion(&self) {
        let add_info = |text: &str| {
            let mut label = Default::default();
            nwg::Label::builder()
                .text(text)
                .parent(&self.window)
                .build(&mut label)
                .expect("Failed to add line");

            let mut lines = self.lines.borrow_mut();
            self.table_layout.add_child_item(nwg::GridLayoutItem::new(
                &label,
                0, lines.len() as u32,
                9, 1
            ));
            lines.push(label);
        };

        fn display_api_version(api_version: u32) -> String {
            format!(
                "{}.{}",
                ash::vk::api_version_major(api_version),
                ash::vk::api_version_minor(api_version)
            )
        }
        
        struct Solution {
            layer: String,
            exclude: bool
        }

        let mut solutions = Vec::with_capacity(self.layers.len());

        if self.conclusion == Conclusion::Healthy {
            add_info("Your computer seems to be perfectly capable of running Vulkan games,");
            add_info("even when all implicit layers are enabled.");
        }
        if self.conclusion == Conclusion::WeirdHealthy {
            add_info("Weird... when no implicit layers are disabled, Vulkan games seem to work fine.");
            add_info("However, problems appear when some layers are disabled.");
        }
        if self.conclusion == Conclusion::Hopeless {
            add_info("It looks like your computer can't run any Vulkan games,");
            add_info("but this doesn't seem to have anything to do with implicit layers.");
            add_info("Perhaps your graphics drivers are missing or outdated?");
        }
        if let Conclusion::WeirdBroken { important_layer, exclude } = &self.conclusion {
            add_info("Your implicit layers are definitely causing problems,");
            add_info("but I didn't find the exact culprit.");
            add_info("The easiest solution seems to be the following:");
            solutions.push(Solution{ layer: important_layer.clone(), exclude: *exclude });
        }
        if let Conclusion::Partial { supported_versions } = &self.conclusion {
            add_info("Your graphics drivers don't seem to support all versions of Vulkan,");
            add_info("but this doesn't seem to have anything to do with your implicit layers.");
            let mut last_line = "The following Vulkan API versions are supported: ".to_string();
            for version in supported_versions {
                last_line += &format!("{}, ", display_api_version(*version));
            }
        }
        if let Conclusion::BrokenLayer { layer } = &self.conclusion {
            add_info(&format!("{} seems to be completely broken, so you should disable it.", layer));
            solutions.push(Solution{ layer: layer.clone(), exclude: true });
        }
        if let Conclusion::PartiallyBrokenLayer { layer, broken_versions } = &self.conclusion {
            add_info(&format!("{} doesn't seem to support all Vulkan versions that your drivers support.", layer));
            let mut next_line = "In particular, it doesn't support Vulkan ".to_string();
            for version in broken_versions {
                next_line += &format!("{}, ", display_api_version(*version));
            }
            add_info("I recommend disabling it.");
            solutions.push(Solution{ layer: layer.clone(), exclude: true });
        }
        if let Conclusion::SymmetricConflict { layers } = &self.conclusion {
            add_info("Some layers are conflicting with each other. I recommend disabling one of them.");
            for layer in layers {
                solutions.push(Solution{ layer: layer.clone(), exclude: true });
            }
        }
        if let Conclusion::AsymmetricConflict { main_offender } = &self.conclusion {
            add_info(&format!("{} conflicts with multiple other layers. I recommend disabling it.", main_offender));
            solutions.push(Solution{ layer: main_offender.clone(), exclude: true });
        }
        if self.conclusion == Conclusion::ComplexConflict {
            add_info("Multiple layers are conflicting with multiple other layers.");
            add_info("I recommend disabling all layers except 1 (pick the one you want to have)");
            for layer in &self.layers {
                solutions.push(Solution{ layer: layer.name.clone(), exclude: false });
            }
        }

        for solution in &solutions {
            let layer = self.layers.iter().find(
                |layer| layer.name == solution.layer
            ).expect("Solution must have a valid layer");

            let description = if solution.exclude {
                format!("Disable {}", solution.layer)
            } else { format!("Disable all layers except {}", solution.layer) };

            let mut label = Default::default();
            nwg::Label::builder()
                .text(&description)
                .parent(&self.window)
                .build(&mut label)
                .expect("Failed to add line");

            let mut lines = self.lines.borrow_mut();
            let mut buttons = self.buttons.borrow_mut();
            let mut handlers = self.handlers.borrow_mut();
            let row = lines.len() as u32;
            self.table_layout.add_child_item(nwg::GridLayoutItem::new(
                &label,
                0, row,
                7, 1
            ));
            lines.push(label);

            let mut disable_button = Default::default();
            nwg::Button::builder()
                .text("Disable")
                .parent(&self.window)
                .build(&mut disable_button)
                .expect("Failed to add disable button");
            let disable_button_handle = disable_button.handle;

            let disable_envs = if solution.exclude {
                vec![layer.disable_environment.clone()]
            } else {
                self.layers.iter()
                    .filter(|candidate| candidate.name != solution.layer)
                    .map(|candidate| candidate.disable_environment.clone())
                    .collect()
            };
            let state_ref = Rc::clone(&self.state);
            let toggle_handler = nwg::bind_event_handler(
                &disable_button.handle, &self.window.handle, move |evt, _evt_data, handle| {
                    if evt == nwg::Event::OnButtonClick && handle == disable_button_handle {
                        let succeeded = disable_envs.iter().all(|env| set_user_environment(env));
                        if succeeded {
                            *state_ref.borrow_mut() = GuiState::AutoFinished(false);
                            nwg::stop_thread_dispatch();
                        } else {
                            *state_ref.borrow_mut() = GuiState::AutoFailed("Failed to disable 1 or more layers".to_string(), false);
                            nwg::stop_thread_dispatch();
                        }
                    }
                }
            );
            handlers.push(toggle_handler);
            self.table_layout.add_child_item(nwg::GridLayoutItem::new(
                &disable_button,
                7, row,
                1, 1
            ));
            buttons.push(disable_button);
            if self.show_break_buttons {
                
                let mut break_button = Default::default();
                nwg::Button::builder()
                    .text("Break")
                    .parent(&self.window)
                    .build(&mut break_button)
                    .expect("Failed to add break button");
                
                let break_button_handle = break_button.handle;
                let state_ref = Rc::clone(&self.state);
                let cloned_conclusion = self.conclusion.clone();
                let cloned_layers = self.layers.clone();
                let files_to_delete = if solution.exclude {
                    vec![layer.settings_path.clone()]
                } else {
                    self.layers.iter()
                        .filter(|candidate| candidate.name != solution.layer)
                        .map(|candidate| candidate.settings_path.clone())
                        .collect()
                };
                let break_handler = nwg::bind_event_handler(
                    &break_button.handle, &self.window.handle, move |evt, _evt_data, handle| {
                        if evt == nwg::Event::OnButtonClick && handle == break_button_handle {
                            let mut error: Option<std::io::Error> = None;
                            for file in &files_to_delete {
                                let delete_result = std::fs::remove_file(file);
                                if let Err(failed_delete) = delete_result {
                                    if failed_delete.kind() == ErrorKind::PermissionDenied {
                                        *state_ref.borrow_mut() = GuiState::AutoConclusion(
                                            cloned_conclusion.clone(),
                                            cloned_layers.clone(),
                                            false
                                        );
                                        nwg::stop_thread_dispatch();
                                        return;
                                    } else {
                                        error = Some(failed_delete);
                                    }
                                }
                            }
                            
                            if let Some(failed) = error {
                                *state_ref.borrow_mut() = GuiState::AutoFailed(failed.to_string(), true);
                                nwg::stop_thread_dispatch();
                            } else {
                                *state_ref.borrow_mut() = GuiState::AutoFinished(true);
                                nwg::stop_thread_dispatch();
                            }
                        }
                    }
                );
                handlers.push(break_handler);
                
                self.table_layout.add_child_item(nwg::GridLayoutItem::new(
                    &break_button,
                    8, row,
                    1, 1
                ));
                buttons.push(break_button);
            }
        }

        add_info("");
        if solutions.is_empty() {
            add_info("Note that this automatic test catches most obviously broken layers,");
            add_info("but NOT subtly broken layers.");
            add_info("If you still have problems, you can always restart this application in Manual mode,");
            add_info("and manually try to find the culprit (or just disable all layers).");
        } else {
            add_info("You can either break or disable layers.");
            add_info("Disabling layers is easiest, but a computer restart may or may not be needed.");
            add_info("Breaking layers usually requires administrator privileges.");
            add_info("When you break a layer, no restart is needed, but it's difficult to recover it later.");
            if !self.show_break_buttons {
                add_info("");
                add_info("If you want to Break layers, you need to restart this application");
                add_info("with administrator privileges.");
            }
        }
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, nwd::NwgUi)]
pub struct AutoFinishedApp {
    #[nwg_control(size: (750, 315), center: true, title: "Automatic: finished", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [AutoFinishedApp::close], OnInit: [AutoFinishedApp::init_description] )]
    pub window: nwg::Window,

    pub did_break: bool,

    #[nwg_control(text: "", size: (500, 250), position: (125, 50))]
    pub description: nwg::Label,

    pub state: Rc<RefCell<GuiState>>
}

impl AutoFinishedApp {

    fn init_description(&self) {
        if self.did_break {
            self.description.set_text(
"All layer(s) were broken successfully.
The next time you launch a game, it's much more likely to succeed.
In the unlikely event there is more than 1 problem,
you can relaunch this application.

You can close this window now."
            );
        } else {
            self.description.set_text(
"All layer(s) were disabled successfully.

Depending on how you launch your games,
you may or may not need to restart your computer.

If you still have problems after restarting your computer,
you can just rerun this application to find potential other problems.
"
            );
        }
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, nwd::NwgUi)]
pub struct AutoFailedApp {
    #[nwg_control(size: (750, 215), center: true, title: "Automatic: finished", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [AutoFailedApp::close], OnInit: [AutoFailedApp::init_description] )]
    pub window: nwg::Window,

    pub error: String,
    pub did_break: bool,

    #[nwg_control(text: "", size: (500, 150), position: (125, 50))]
    pub description: nwg::Label,

    pub state: Rc<RefCell<GuiState>>
}

impl AutoFailedApp {

    fn init_description(&self) {
        if self.did_break {
            self.description.set_text(&format!("Failed to break 1 or more layers:\n{}", self.error));
        } else {
            self.description.set_text("Disabling 1 or more layers failed for some reason");
        }
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}
