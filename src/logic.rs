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

pub fn draw_conclusion(results: &TestResults) -> Conclusion {
    let mut all_results = Vec::with_capacity(2 + 2 * results.isolation_results.len());
    all_results.push(results.clean_result.clone());
    all_results.push(results.default_result.clone());
    for (_, result) in &results.isolation_results {
        all_results.push(result.clone());
    }
    for (_, result) in &results.exclude_results {
        all_results.push(result.clone());
    }

    if all_results.iter().all(|trial| trial.all_succeeded()) {
        return Conclusion::Healthy;
    }
    if all_results.iter().all(|trial| trial.all_failed()) {
        return Conclusion::Hopeless;
    }

    // When this code is reached, there must be at least 1 failed and at least 1
    // succeeded trial

    let mut supported_versions = Vec::with_capacity(4);
    let mut unsupported_versions = Vec::with_capacity(4);
    for version in [ash::vk::API_VERSION_1_0, ash::vk::API_VERSION_1_1, ash::vk::API_VERSION_1_2, ash::vk::API_VERSION_1_3] {
        if all_results.iter().all(|trial| trial.succeeded(version)) {
            supported_versions.push(version);
        }
        if all_results.iter().all(|trial| !trial.succeeded(version)) {
            unsupported_versions.push(version);
        }
    }

    if unsupported_versions.len() + supported_versions.len() == 4 {
        return Conclusion::Partial { supported_versions }
    }

    // When this code is reached, not all layers support exactly the same Vulkan API versions.
    // Maybe, some layer doesn't like a particular Vulkan version.
    // Maybe, some layer doesn't support any Vulkan version.

    if results.default_result.succeeded_except(&unsupported_versions) {
        return Conclusion::WeirdHealthy;
    }

    // When this code is reached, the default trials didn't succeed

    if !results.clean_result.succeeded_except(&unsupported_versions) {
        for (layer, layer_results) in &results.exclude_results {
            if layer_results.succeeded_except(&unsupported_versions) {
                return Conclusion::WeirdBroken { important_layer: layer.clone(), exclude: true };
            }
        }
        for (layer, layer_results) in &results.isolation_results {
            if layer_results.succeeded_except(&unsupported_versions) {
                return Conclusion::WeirdBroken { important_layer: layer.clone(), exclude: false };
            }
        }
        unreachable!();
    }

    // When this code is reached, the clean trials succeeded

    for (layer, layer_results) in &results.isolation_results {
        if !layer_results.succeeded_except(&unsupported_versions) {
            return if layer_results.all_failed() {
                Conclusion::BrokenLayer { layer: layer.clone() }
            } else {
                Conclusion::PartiallyBrokenLayer {
                    layer: layer.clone(),
                    broken_versions: [
                        ash::vk::API_VERSION_1_0, ash::vk::API_VERSION_1_1, ash::vk::API_VERSION_1_2, ash::vk::API_VERSION_1_3
                    ].into_iter().filter(
                        |api_version| !unsupported_versions.contains(api_version)
                            && !layer_results.succeeded(*api_version)
                    ).collect()
                }
            }
        }
    }

    // When this code is reached, all layers work in isolation, so there must be some conflict

    let num_succeeded_exclude_trials = results.exclude_results.iter().filter(|(_, layer_results)| {
        layer_results.succeeded_except(&unsupported_versions)
    }).count();

    if num_succeeded_exclude_trials == 0 {
        return Conclusion::ComplexConflict;
    }

    if num_succeeded_exclude_trials == 1 {
        for (layer, layer_results) in &results.exclude_results {
            if layer_results.succeeded_except(&unsupported_versions) {
                return Conclusion::AsymmetricConflict { main_offender: layer.clone() };
            }
        }
        unreachable!()
    }

    // When this code is reached, there are multiple possibilities to resolve the conflict

    let conflicting_layers = results.exclude_results.iter().filter(|(_, layer_results)| {
        layer_results.succeeded_except(&unsupported_versions)
    }).map(|(layer, _)| layer.clone()).collect();
    Conclusion::SymmetricConflict { layers: conflicting_layers }
}

#[cfg(test)]
mod tests {
    use ash::vk;
    use crate::definitions::{Conclusion, TestResults, TrialResult, VersionedTrialResults};
    use crate::logic::draw_conclusion;

    fn failed_all() -> VersionedTrialResults {
        VersionedTrialResults {
            vk10: TrialResult { exit_code: 1234, output: "failed1234".to_string() },
            vk11: TrialResult { exit_code: 1234, output: "failed1234".to_string() },
            vk12: TrialResult { exit_code: 1234, output: "failed1234".to_string() },
            vk13: TrialResult { exit_code: 1234, output: "failed1234".to_string() },
        }
    }

    fn succeeded_all() -> VersionedTrialResults {
        VersionedTrialResults {
            vk10: TrialResult { exit_code: 0, output: "".to_string() },
            vk11: TrialResult { exit_code: 0, output: "".to_string() },
            vk12: TrialResult { exit_code: 0, output: "".to_string() },
            vk13: TrialResult { exit_code: 0, output: "".to_string() },
        }
    }

    #[test]
    fn test_draw_conclusion_healthy_no_layers() {
        let results = TestResults {
            default_result: succeeded_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![],
            isolation_results: vec![],
        };
        assert_eq!(Conclusion::Healthy, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_healthy_1_layer() {
        let results = TestResults {
            default_result: succeeded_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![("dummy".to_string(), succeeded_all())],
            isolation_results: vec![("dummy".to_string(), succeeded_all())],
        };
        assert_eq!(Conclusion::Healthy, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_healthy_2_layers() {
        let results = TestResults {
            default_result: succeeded_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::Healthy, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_hopeless_no_layers() {
        let results = TestResults {
            default_result: failed_all(),
            clean_result: failed_all(),
            exclude_results: vec![],
            isolation_results: vec![],
        };
        assert_eq!(Conclusion::Hopeless, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_hopeless_1_layer() {
        let results = TestResults {
            default_result: failed_all(),
            clean_result: failed_all(),
            exclude_results: vec![("dummy".to_string(), failed_all())],
            isolation_results: vec![("dummy".to_string(), failed_all())],
        };
        assert_eq!(Conclusion::Hopeless, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_hopeless_2_layers() {
        let results = TestResults {
            default_result: failed_all(),
            clean_result: failed_all(),
            exclude_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer2".to_string(), failed_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer2".to_string(), failed_all())
            ],
        };
        assert_eq!(Conclusion::Hopeless, draw_conclusion(&results));
    }
    
    fn without_vk12_support() -> VersionedTrialResults {
        VersionedTrialResults {
            vk10: TrialResult { exit_code: 0, output: "".to_string() },
            vk11: TrialResult { exit_code: 0, output: "".to_string() },
            vk12: TrialResult { exit_code: 21000, output: "not happening".to_string() },
            vk13: TrialResult { exit_code: 0, output: "".to_string() },
        }
    }

    #[test]
    fn test_draw_conclusion_partial_no_layers() {
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: without_vk12_support(),
            exclude_results: vec![],
            isolation_results: vec![],
        };
        assert_eq!(Conclusion::Partial { supported_versions: vec![
            vk::API_VERSION_1_0, vk::API_VERSION_1_1, vk::API_VERSION_1_3
        ] }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_partial_1_layer() {
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: without_vk12_support(),
            exclude_results: vec![("dummy".to_string(), without_vk12_support())],
            isolation_results: vec![("dummy".to_string(), without_vk12_support())],
        };
        assert_eq!(Conclusion::Partial { supported_versions: vec![
            vk::API_VERSION_1_0, vk::API_VERSION_1_1, vk::API_VERSION_1_3
        ] }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_partial_2_layers() {
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: without_vk12_support(),
            exclude_results: vec![
                ("layer1".to_string(), without_vk12_support()),
                ("layer2".to_string(), without_vk12_support())
            ],
            isolation_results: vec![
                ("layer1".to_string(), without_vk12_support()),
                ("layer2".to_string(), without_vk12_support())
            ],
        };
        assert_eq!(Conclusion::Partial { supported_versions: vec![
            vk::API_VERSION_1_0, vk::API_VERSION_1_1, vk::API_VERSION_1_3
        ] }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_weird_healthy_1_layer_without_vk12() {
        let results = TestResults {
            default_result: succeeded_all(),
            clean_result: without_vk12_support(),
            exclude_results: vec![("dummy".to_string(), without_vk12_support())],
            isolation_results: vec![("dummy".to_string(), succeeded_all())],
        };
        assert_eq!(Conclusion::WeirdHealthy, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_weird_healthy_1_layer_total_failure() {
        let results = TestResults {
            default_result: succeeded_all(),
            clean_result: failed_all(),
            exclude_results: vec![("dummy".to_string(), failed_all())],
            isolation_results: vec![("dummy".to_string(), succeeded_all())],
        };
        assert_eq!(Conclusion::WeirdHealthy, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_weird_healthy_2_layers() {
        let results = TestResults {
            default_result: succeeded_all(),
            clean_result: without_vk12_support(),
            exclude_results: vec![
                ("layer1".to_string(), without_vk12_support()),
                ("layer2".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), without_vk12_support())
            ],
        };
        assert_eq!(Conclusion::WeirdHealthy, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_weird_broken_2_layers() {
        // layer1 is required and layer2 fails on Vulkan 1.2
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: failed_all(),
            exclude_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer2".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), failed_all())
            ],
        };
        assert_eq!(Conclusion::WeirdBroken {
            important_layer: "layer2".to_string(), exclude: true
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_weird_broken_3_layers_1_broken() {
        // layer1 fails on Vulkan 1.2
        // layer2 is not important
        // layer3 is required
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: failed_all(),
            exclude_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), without_vk12_support()),
                ("layer3".to_string(), failed_all()),
            ],
            isolation_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer2".to_string(), failed_all()),
                ("layer3".to_string(), succeeded_all()),
            ],
        };
        assert_eq!(Conclusion::WeirdBroken {
            important_layer: "layer1".to_string(), exclude: true
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_weird_broken_3_layers_1_and_2_broken() {
        // layer1 fails on Vulkan 1.2
        // layer2 always fails
        // layer3 is required
        let results = TestResults {
            default_result: failed_all(),
            clean_result: failed_all(),
            exclude_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer2".to_string(), without_vk12_support()),
                ("layer3".to_string(), failed_all()),
            ],
            isolation_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer2".to_string(), failed_all()),
                ("layer3".to_string(), succeeded_all()),
            ],
        };
        assert_eq!(Conclusion::WeirdBroken {
            important_layer: "layer3".to_string(), exclude: false
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_broken_layer_1_layer() {
        let results = TestResults {
            default_result: failed_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![("broken".to_string(), succeeded_all())],
            isolation_results: vec![("broken".to_string(), failed_all())],
        };
        assert_eq!(Conclusion::BrokenLayer { layer: "broken".to_string() }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_broken_layer_2_layers() {
        // layer2 doesn't support Vulkan 1.2
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), without_vk12_support()),
                ("layer2".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), without_vk12_support())
            ],
        };
        assert_eq!(Conclusion::PartiallyBrokenLayer { 
            layer: "layer2".to_string(),
            broken_versions: vec![vk::API_VERSION_1_2],
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_symmetric_conflict_2_layers() {
        let results = TestResults {
            default_result: failed_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::SymmetricConflict {
            layers: vec!["layer1".to_string(), "layer2".to_string()]
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_symmetric_conflict_3_layers() {
        let results = TestResults {
            default_result: failed_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::SymmetricConflict {
            layers: vec!["layer1".to_string(), "layer2".to_string(), "layer3".to_string()]
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_symmetric_conflict_3_layers_1_irrelevant() {
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), without_vk12_support()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::SymmetricConflict {
            layers: vec!["layer2".to_string(), "layer3".to_string()]
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_asymmetric_conflict_3_layers() {
        // layer2 is the main offender
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), without_vk12_support()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), without_vk12_support())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::AsymmetricConflict {
            main_offender: "layer2".to_string()
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_asymmetric_conflict_4_layers() {
        // layer2 is the main offender
        let results = TestResults {
            default_result: failed_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer3".to_string(), failed_all()),
                ("layer4".to_string(), failed_all()),
                ("layer2".to_string(), succeeded_all())
            ],
            isolation_results: vec![
                ("layer4".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all()),
                ("layer1".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::AsymmetricConflict {
            main_offender: "layer2".to_string()
        }, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_complex_conflict_3_layers() {
        let results = TestResults {
            default_result: without_vk12_support(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), without_vk12_support()),
                ("layer2".to_string(), without_vk12_support()),
                ("layer3".to_string(), without_vk12_support())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::ComplexConflict, draw_conclusion(&results));
    }

    #[test]
    fn test_draw_conclusion_complex_conflict_4_layers() {
        let results = TestResults {
            default_result: failed_all(),
            clean_result: succeeded_all(),
            exclude_results: vec![
                ("layer1".to_string(), failed_all()),
                ("layer3".to_string(), failed_all()),
                ("layer4".to_string(), failed_all()),
                ("layer2".to_string(), failed_all())
            ],
            isolation_results: vec![
                ("layer1".to_string(), succeeded_all()),
                ("layer2".to_string(), succeeded_all()),
                ("layer3".to_string(), succeeded_all()),
                ("layer4".to_string(), succeeded_all())
            ],
        };
        assert_eq!(Conclusion::ComplexConflict, draw_conclusion(&results));
    }
}
