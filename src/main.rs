mod definitions;
mod logic;
mod registry;
mod test_app;

use test_app::maybe_run_test_app;
use logic::run_tests;

fn main() {
    maybe_run_test_app();
    run_tests();
}
