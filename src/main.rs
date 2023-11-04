#![windows_subsystem = "windows"]

mod definitions;
mod logic;
mod registry;
mod test_app;

use test_app::maybe_run_test_app;
use logic::run_all_trials;
use nwd::NwgUi;
use nwg::{CheckBoxState, NativeUi};
use std::cell::RefCell;
use std::rc::Rc;
use crate::definitions::{ImplicitLayer, TestResults, VersionedTrialResults};
use crate::registry::{get_global_environment_keys, get_implicit_layers, is_enabled, remove_user_environment, set_user_environment};

fn main() {
    maybe_run_test_app();

    nwg::init().expect("Failed to init Native Windows GUI");

    let state = Rc::new(RefCell::new(GuiState::Initial));

    loop {
        let cloned_state = state.borrow().clone();

        if cloned_state == GuiState::Exit {
            break;
        }
        if cloned_state == GuiState::Initial {
            let _ui = FixerApp::build_ui(FixerApp { state: Rc::clone(&state), ..Default::default() }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }
        if cloned_state == GuiState::Manual {
            let _ui = ManualApp::build_ui(ManualApp { state: Rc::clone(&state), ..Default::default() }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }
        if cloned_state == GuiState::AutoLayerList {
            let _ui = AutoLayerApp::build_ui(AutoLayerApp { state: Rc::clone(&state), ..Default::default() }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }

        if let GuiState::AutoResultsTable(results) = cloned_state {
            let _ui = AutoResultsApp::build_ui(AutoResultsApp { state: Rc::clone(&state), results, ..Default::default() }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }
    }
}

#[derive(Eq, PartialEq, Clone, Default)]
enum GuiState {
    #[default]
    Initial,
    Manual,
    AutoLayerList,
    AutoResultsTable(TestResults),
    Exit
}

#[derive(Default, NwgUi)]
pub struct FixerApp {
    #[nwg_control(size: (750, 315), center: true, title: "vk-fixer", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [FixerApp::say_goodbye] )]
    window: nwg::Window,

    #[nwg_control(text: "Automatic mode", size: (300, 50), position: (50, 200))]
    #[nwg_events( OnButtonClick: [FixerApp::start_automatic_mode] )]
    automatic_button: nwg::Button,

    #[nwg_control(text: "Manual mode", size: (300, 50), position: (400, 200))]
    #[nwg_events( OnButtonClick: [FixerApp::start_manual_mode] )]
    manual_button: nwg::Button,

    state: Rc<RefCell<GuiState>>
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
        *self.state.borrow_mut() = GuiState::Manual;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, NwgUi)]
pub struct ManualApp {
    #[nwg_events( OnWindowClose: [ManualApp::close], OnInit: [ManualApp::init_layers] )]
    #[nwg_control(size: (750, 700), center: true, title: "Manual layer selection", flags: "MAIN_WINDOW|VISIBLE")]
    window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 0, margin: [50, 50, 50, 50])]
    layout: nwg::GridLayout,

    layer_names: Rc<RefCell<Vec<nwg::CheckBox>>>,
    layer_info: RefCell<Vec<nwg::Label>>,
    handlers: RefCell<Vec<nwg::EventHandler>>,

    state: Rc<RefCell<GuiState>>
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
            self.layout.add_child(0, (self.layer_names.borrow().len() + layer_info.len()) as u32, &label);
            layer_info.push(label);
        };

        add_info("Disable layers by checking the corresponding boxes.");
        add_info("Note: depending on the game and how the game is launched,");
        add_info("a computer restart may or may not be needed.");
        add_info("");
        for layer in layers {

            let mut layer_box = Default::default();
            let is_disabled = env.user.contains(&layer.disable_environment);
            nwg::CheckBox::builder()
                .text(&layer.name)
                .check_state(if is_disabled { CheckBoxState::Checked } else { CheckBoxState::Unchecked })
                .parent(&self.window)
                .build(&mut layer_box)
                .expect("Failed to add layer checkbox");

            let toggle_handler = layer_box.handle;
            let disable_env = layer.disable_environment.clone();
            let layer_names_ref = Rc::clone(&self.layer_names);
            let handler = nwg::bind_event_handler(
                &layer_box.handle, &self.window.handle, move |evt, _evt_data, handle| {
                    if evt == nwg::Event::OnButtonClick && handle == toggle_handler {
                        let mut is_disabled = get_global_environment_keys().user.contains(&disable_env);
                        if is_disabled {
                            is_disabled = !remove_user_environment(&disable_env);
                        } else {
                            is_disabled = set_user_environment(&disable_env);
                        }

                        let layer_boxes = layer_names_ref.borrow_mut();
                        for layer_box in &*layer_boxes {
                            if layer_box.handle == toggle_handler {
                                layer_box.set_check_state(if is_disabled { CheckBoxState::Checked } else { CheckBoxState::Unchecked });
                            }
                        }
                    }
            });
            self.handlers.borrow_mut().push(handler);

            let mut layer_names = self.layer_names.borrow_mut();
            self.layout.add_child(0, (layer_names.len() + self.layer_info.borrow().len()) as u32, &layer_box);
            layer_names.push(layer_box);
            drop(layer_names);

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

#[derive(Default, NwgUi)]
pub struct AutoLayerApp {
    #[nwg_events( OnWindowClose: [AutoLayerApp::close], OnInit: [AutoLayerApp::init_layers] )]
    #[nwg_control(size: (650, 400), center: true, title: "Automatic: layer list", flags: "MAIN_WINDOW|VISIBLE")]
    window: nwg::Window,

    #[nwg_control(text: "Run trials...", size: (300, 50), position: (150, 325))]
    #[nwg_events( OnButtonClick: [AutoLayerApp::run_trials] )]
    trials_button: nwg::Button,

    #[nwg_layout(parent: window, spacing: 0, margin: [0, 50, 100, 50])]
    layout: nwg::GridLayout,

    layer_info: RefCell<Vec<nwg::Label>>,

    layer_list: RefCell<Vec<ImplicitLayer>>,

    state: Rc<RefCell<GuiState>>
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
        let results = run_all_trials(self.layer_list.borrow().as_slice());
        *self.state.borrow_mut() = GuiState::AutoResultsTable(results);
        nwg::stop_thread_dispatch();
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, NwgUi)]
pub struct AutoResultsApp {
    #[nwg_events( OnWindowClose: [AutoResultsApp::close], OnInit: [AutoResultsApp::init_results_table] )]
    #[nwg_control(size: (1200, 700), center: true, title: "Automatic: layer list", flags: "MAIN_WINDOW|VISIBLE")]
    window: nwg::Window,

    #[nwg_control(text: "Jump to conclusions", size: (300, 50), position: (150, 125))]
    #[nwg_events( OnButtonClick: [AutoResultsApp::jump_to_conclusions] )]
    conclusions_button: nwg::Button,

    #[nwg_layout(parent: window, spacing: 0, margin: [0, 50, 600, 50])]
    info_layout: nwg::GridLayout,

    #[nwg_layout(parent: window, spacing: 0, margin: [200, 0, 0, 0])]
    table_layout: nwg::GridLayout,

    results: TestResults,

    results_table: RefCell<Vec<nwg::Label>>,
    info_labels: RefCell<Vec<nwg::Label>>,

    state: Rc<RefCell<GuiState>>
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
    }

    fn close(&self) {
        *self.state.borrow_mut() = GuiState::Exit;
        nwg::stop_thread_dispatch();
    }
}
