use std::collections::HashMap;

use rand::{rngs::StdRng, thread_rng, Rng, SeedableRng};
use triton_vm::{BFieldElement, NonDeterminism};

use crate::{
    linker::{execute_bench, link_for_isolated_run},
    snippet::{BasicSnippet, RustShadow},
    snippet_bencher::{write_benchmarks, BenchmarkCase, BenchmarkResult},
    test_helpers::test_rust_equivalence_given_complete_state,
};

/// A Function can modify the top of the stack, and can read and
/// extend memory. Specifically: any memory writes have to happen
/// to addresses larger than the dynamic memory allocator and the
/// dynamic memory allocator value has to be updated accordingly.
pub trait Function: BasicSnippet {
    fn rust_shadow(
        &self,
        stack: &mut Vec<BFieldElement>,
        memory: &mut HashMap<BFieldElement, BFieldElement>,
    );

    fn pseudorandom_initial_state(
        &self,
        seed: [u8; 32],
        bench_case: Option<BenchmarkCase>,
    ) -> (Vec<BFieldElement>, HashMap<BFieldElement, BFieldElement>);
}

pub struct ShadowedFunction<F: Function + Clone + 'static> {
    pub function: F,
}

impl<F: Function + Clone + 'static> ShadowedFunction<F> {
    pub fn new(function: F) -> Self {
        Self { function }
    }
}

impl<F> RustShadow for ShadowedFunction<F>
where
    F: Function + Clone + 'static,
{
    fn rust_shadow_wrapper(
        &self,
        _stdin: &[BFieldElement],
        _nondeterminism: &triton_vm::NonDeterminism<BFieldElement>,
        stack: &mut Vec<BFieldElement>,
        memory: &mut HashMap<BFieldElement, BFieldElement>,
    ) -> Vec<BFieldElement> {
        self.function.rust_shadow(stack, memory);
        vec![]
    }

    fn test(&self) {
        let num_states = 5;
        let mut rng = thread_rng();

        for _ in 0..num_states {
            let seed: [u8; 32] = rng.gen();
            println!(
                "testing {} common case with seed: {:x?}",
                self.function.entrypoint(),
                seed
            );
            let (stack, memory) = self.function.pseudorandom_initial_state(seed, None);

            let stdin = vec![];
            test_rust_equivalence_given_complete_state(
                self,
                &stack,
                &stdin,
                &NonDeterminism::new(vec![]),
                &memory,
                1,
                None,
            );
        }
    }

    fn bench(&self) {
        let mut rng: StdRng = SeedableRng::from_seed(
            hex::decode("73a24b6b8b32e4d7d563a4d9a85f476573a24b6b8b32e4d7d563a4d9a85f4765")
                .unwrap()
                .try_into()
                .unwrap(),
        );
        let mut benchmarks = Vec::with_capacity(2);

        for bench_case in [BenchmarkCase::CommonCase, BenchmarkCase::WorstCase] {
            let (stack, memory) = self
                .function
                .pseudorandom_initial_state(rng.gen(), Some(bench_case));
            let program = link_for_isolated_run(&self.function, 1);
            let execution_result = execute_bench(
                &program,
                &stack,
                vec![],
                NonDeterminism::new(vec![]),
                &memory,
                Some(1),
            );
            let benchmark = BenchmarkResult {
                name: self.function.entrypoint(),
                clock_cycle_count: execution_result.cycle_count,
                hash_table_height: execution_result.hash_table_height,
                u32_table_height: execution_result.u32_table_height,
                case: bench_case,
            };
            benchmarks.push(benchmark);
        }

        write_benchmarks(benchmarks);
    }

    fn inner(&self) -> Box<dyn BasicSnippet> {
        Box::new(self.function.clone())
    }
}
