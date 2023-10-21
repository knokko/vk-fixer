#![windows_subsystem = "windows"]

mod definitions;
mod logic;
mod registry;
mod test_app;

use test_app::maybe_run_test_app;
use logic::run_tests;
use nwd::NwgUi;
use nwg::{CheckBoxState, NativeUi};
use std::cell::RefCell;
use std::rc::Rc;
use crate::registry::{get_global_environment_keys, get_implicit_layers, remove_user_environment, set_user_environment};

fn main() {
    maybe_run_test_app();

    nwg::init().expect("Failed to init Native Windows GUI");

    let state = Rc::new(RefCell::new(GuiState::Initial));

    loop {
        if *(*state).borrow() == GuiState::Exit {
            break;
        }
        if *(*state).borrow() == GuiState::Initial {
            let _ui = FixerApp::build_ui(FixerApp { state: Rc::clone(&state), ..Default::default() }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }
        if *(*state).borrow() == GuiState::Manual {
            let _ui = ManualApp::build_ui(ManualApp { state: Rc::clone(&state), ..Default::default() }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }
    }
}

#[derive(Eq, PartialEq, Clone, Default)]
enum GuiState {
    #[default]
    Initial,
    Manual,
    Exit
}

#[derive(Default, NwgUi)]
pub struct FixerApp {
    #[nwg_control(size: (750, 315), position: (300, 300), title: "vk-fixer", flags: "WINDOW|VISIBLE")]
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
        println!("here we go");
        run_tests();
    }

    fn start_manual_mode(&self) {
        *self.state.borrow_mut() = GuiState::Manual;
        nwg::stop_thread_dispatch();
    }
}

#[derive(Default, NwgUi)]
pub struct ManualApp {
    #[nwg_events( OnWindowClose: [ManualApp::close], OnInit: [ManualApp::init_layers] )]
    #[nwg_control(size: (750, 700), position: (300, 50), title: "Manual layer selection", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnKeyEnter: [ManualApp::start_automatic_mode])]
    window: nwg::Window,

    #[nwg_layout(parent: window, spacing: 0, margin: [50, 50, 50, 50])]
    #[nwg_events( OnMousePress: [ManualApp::start_automatic_mode])]
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

        add_info("Disable layers by checking the corresponding boxes");
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
