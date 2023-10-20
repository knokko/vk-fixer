use crate::definitions::*;
use crate::test_app::{await_test_app, spawn_test_app};
use crate::registry::{get_implicit_layers, is_enabled};

pub fn run_tests() {
    let (all_implicit_layers, errors) = get_implicit_layers();
    let implicit_layers = all_implicit_layers.into_iter().filter(
        |layer| is_enabled(layer)
    ).collect::<Vec<_>>();

    if errors.len() > 0 {
        println!("Errors are:");
        for error in errors {
            println!("  {}", error);
        }
        println!();
    }

    println!("Implicit layers are:");
    for layer in &implicit_layers {
        println!("  {}", layer.name);
    }

    println!("test results are {:?}", run_all_trials(&implicit_layers));
}

fn run_all_trials(layers: &[ImplicitLayer]) -> TestResults {
    let default_trial = spawn_test_app(&[]);
    let clean_trial = spawn_test_app(&layers.iter().map(
        |layer| layer.disable_environment.as_str()
    ).collect::<Vec<_>>());
    let exclude_trials = layers.iter().map(
        |layer| (layer.name.clone(), spawn_test_app(&[layer.disable_environment.as_str()]))
    );
    let isolation_trials = layers.iter().map(
        |only_layer| (only_layer.name.clone(), spawn_test_app(&layers.iter().filter(
            |other_layer| other_layer != &only_layer
        ).map(|other_layer| other_layer.disable_environment.as_str()).collect::<Vec<_>>()))
    );
    TestResults {
        default_result: await_test_app(default_trial),
        clean_result: await_test_app(clean_trial),
        exclude_results: exclude_trials.map(|trial| (trial.0, await_test_app(trial.1))).collect(),
        isolation_results: isolation_trials.map(|trial| (trial.0, await_test_app(trial.1))).collect(),
    }
}
