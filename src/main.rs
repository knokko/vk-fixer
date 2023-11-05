#![windows_subsystem = "windows"]

mod definitions;
mod gui;
mod logic;
mod registry;
mod test_app;

use test_app::maybe_run_test_app;
use nwg::NativeUi;
use std::cell::RefCell;
use std::rc::Rc;
use gui::*;

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
            let _ui = FixerApp::build_ui(FixerApp {
                state: Rc::clone(&state),
                ..Default::default()
            }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }

        if let GuiState::Manual(show_break_buttons) = cloned_state {
            let _ui = ManualApp::build_ui(ManualApp {
                state: Rc::clone(&state),
                show_break_buttons,
                ..Default::default()
            }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }

        if cloned_state == GuiState::AutoLayerList {
            let _ui = AutoLayerApp::build_ui(AutoLayerApp {
                state: Rc::clone(&state),
                ..Default::default()
            }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }

        if let GuiState::AutoResultsTable(results, layers) = &cloned_state {
            let _ui = AutoResultsApp::build_ui(AutoResultsApp {
                state: Rc::clone(&state),
                layers: layers.clone(),
                results: results.clone(),
                ..Default::default()
            }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }

        if let GuiState::AutoConclusion(conclusion, layers, show_break_buttons) = &cloned_state {
            let _ui = AutoConclusionApp::build_ui(AutoConclusionApp {
                state: Rc::clone(&state),
                conclusion: conclusion.clone(),
                layers: layers.clone(),
                show_break_buttons: *show_break_buttons,
                ..Default::default()
            }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }

        if let GuiState::AutoFinished(did_break) = &cloned_state {
            let _ui = AutoFinishedApp::build_ui(AutoFinishedApp {
                state: Rc::clone(&state),
                did_break: *did_break,
                ..Default::default()
            }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }

        if let GuiState::AutoFailed(error, did_break) = &cloned_state {
            let _ui = AutoFailedApp::build_ui(AutoFailedApp {
                state: Rc::clone(&state),
                error: error.clone(),
                did_break: *did_break,
                ..Default::default()
            }).expect("Failed to build UI");
            nwg::dispatch_thread_events();
        }
    }
}
