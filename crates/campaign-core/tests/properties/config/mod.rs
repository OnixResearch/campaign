//! Deterministic runner: fixed seed and algorithm, no failure-persistence files, bounded cases.
//!
//! Every field is explicit, so no `PROPTEST_*` environment variable changes a run.

/// Fixed `ChaCha` seed shared by every property; the same build always generates the same cases.
pub(crate) const RNG_SEED: u64 = 0x4341_4d50_4149_474e;
/// Cases for properties over directly generated campaign inputs.
pub(crate) const INPUT_CASES: u32 = 256;
/// Cases for properties over generated control histories (each rebuilds every prefix).
pub(crate) const HISTORY_CASES: u32 = 128;
/// Shrink iterations after a failure; bounded so a failing run still ends quickly.
const SHRINK_ITERATIONS: u32 = 512;
/// Zero disables the wall-clock shrink limit, keeping shrinking a pure function of the seed.
const SHRINK_TIME_LIMIT_MS: u32 = 0;
/// Local and global filter rejections; the suite's strategies never filter.
const MAX_LOCAL_REJECTS: u32 = 1024;
const MAX_GLOBAL_REJECTS: u32 = 1024;
/// Regenerations allowed for dependent (`prop_flat_map`) strategies.
const MAX_FLAT_MAP_REGENS: u32 = 1024;
/// Size range for collection strategies without an explicit size.
const DEFAULT_SIZE_RANGE: usize = 16;
/// No per-case runner output.
const VERBOSITY: u32 = 0;

pub(crate) fn runner(cases: u32) -> proptest::test_runner::TestRunner {
    debug_assert!(cases > 0);
    proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases,
        max_local_rejects: MAX_LOCAL_REJECTS,
        max_global_rejects: MAX_GLOBAL_REJECTS,
        max_flat_map_regens: MAX_FLAT_MAP_REGENS,
        failure_persistence: None,
        source_file: None,
        test_name: None,
        max_shrink_time: SHRINK_TIME_LIMIT_MS,
        max_shrink_iters: SHRINK_ITERATIONS,
        max_default_size_range: DEFAULT_SIZE_RANGE,
        result_cache: proptest::test_runner::noop_result_cache,
        verbose: VERBOSITY,
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(RNG_SEED),
        _non_exhaustive: (),
    })
}

/// Runs `property` over `cases` deterministic values of `strategy`, reporting the minimal failure.
pub(crate) fn check<S>(
    cases: u32,
    strategy: &S,
    property: impl Fn(S::Value) -> Result<(), String>,
) -> Result<(), String>
where
    S: proptest::strategy::Strategy,
{
    runner(cases)
        .run(strategy, |value| {
            property(value).map_err(proptest::test_runner::TestCaseError::fail)
        })
        .map_err(|error| format!("{error}"))
}
