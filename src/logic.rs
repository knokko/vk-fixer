use crate::definitions::*;
use crate::test_app::{await_test_apps, spawn_test_apps};

pub fn run_all_trials(layers: &[ImplicitLayer]) -> TestResults {
    let default_trial = spawn_test_apps(&[]);
    let clean_trial = spawn_test_apps(&layers.iter().map(
        |layer| layer.disable_environment.as_str()
    ).collect::<Vec<_>>());
    let exclude_trials = layers.iter().map(
        |layer| (layer.name.clone(), spawn_test_apps(&[layer.disable_environment.as_str()]))
    );
    let isolation_trials = layers.iter().map(
        |only_layer| (only_layer.name.clone(), spawn_test_apps(&layers.iter().filter(
            |other_layer| other_layer != &only_layer
        ).map(|other_layer| other_layer.disable_environment.as_str()).collect::<Vec<_>>()))
    );
    TestResults {
        default_result: await_test_apps(default_trial),
        clean_result: await_test_apps(clean_trial),
        exclude_results: exclude_trials.map(|trial| (trial.0, await_test_apps(trial.1))).collect(),
        isolation_results: isolation_trials.map(|trial| (trial.0, await_test_apps(trial.1))).collect(),
    }
}
