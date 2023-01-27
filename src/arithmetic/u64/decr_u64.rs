use std::collections::HashMap;

use num::One;
use twenty_first::amount::u32s::U32s;
use twenty_first::shared_math::b_field_element::BFieldElement;
use twenty_first::util_types::algebraic_hasher::Hashable;

use crate::library::Library;
use crate::snippet::Snippet;
use crate::{get_init_tvm_stack, push_hashable, ExecutionState};

pub struct DecrU64();

impl Snippet for DecrU64 {
    fn inputs() -> Vec<&'static str> {
        vec!["value_hi", "value_lo"]
    }

    fn outputs() -> Vec<&'static str> {
        vec!["(value - 1)_hi", "(value - 1)_lo"]
    }

    fn crash_conditions() -> Vec<&'static str> {
        vec!["value == 0"]
    }

    fn gen_input_states() -> Vec<ExecutionState> {
        let values = vec![
            // U32s::<2>::zero(),
            U32s::<2>::new([0, 14]),
            U32s::<2>::new([u32::MAX, 13]),
        ];
        values
            .into_iter()
            .map(|value| {
                let mut stack = get_init_tvm_stack();
                push_hashable(&mut stack, &value);
                ExecutionState::with_stack(stack)
            })
            .collect()
    }

    fn stack_diff() -> isize {
        0
    }

    fn entrypoint() -> &'static str {
        "decr_u64"
    }

    fn function_body(_library: &mut Library) -> String {
        let entrypoint = Self::entrypoint();
        const U32_MAX: &str = "4294967295";

        format!(
            "
            {entrypoint}:
                push -1
                add
                dup0
                push -1
                eq
                skiz
                    call {entrypoint}_carry
                return

            {entrypoint}_carry:
                pop
                push -1
                add
                dup0
                push -1
                eq
                push 0
                eq
                assert
                push {U32_MAX}
                return
            "
        )
    }

    fn rust_shadowing(
        stack: &mut Vec<BFieldElement>,
        _std_in: Vec<BFieldElement>,
        _secret_in: Vec<BFieldElement>,
        _memory: &mut HashMap<BFieldElement, BFieldElement>,
    ) {
        let a: u32 = stack.pop().unwrap().try_into().unwrap();
        let b: u32 = stack.pop().unwrap().try_into().unwrap();
        let ab = U32s::<2>::new([a, b]);
        let ab_incr = ab - U32s::one();
        let mut res = ab_incr.to_sequence();
        for _ in 0..res.len() {
            stack.push(res.pop().unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use num::Zero;
    use rand::Rng;

    use crate::snippet_bencher::bench_and_write;
    use crate::test_helpers::{rust_tasm_equivalence_prop, rust_tasm_equivalence_prop_new};
    use crate::{get_init_tvm_stack, push_hashable};

    use super::*;

    #[test]
    fn decr_u64_test() {
        rust_tasm_equivalence_prop_new::<DecrU64>();
    }

    #[test]
    fn decr_u64_benchmark() {
        bench_and_write::<DecrU64>();
    }

    #[test]
    #[should_panic]
    fn decr_u64_negative_tasm_test() {
        let mut stack = get_init_tvm_stack();
        push_hashable(&mut stack, &U32s::<2>::zero());
        DecrU64::run_tasm_old(&mut stack, vec![], vec![], &mut HashMap::default(), 0);
    }

    #[test]
    #[should_panic]
    fn decr_u64_negative_rust_test() {
        let mut stack = get_init_tvm_stack();
        push_hashable(&mut stack, &U32s::<2>::zero());
        DecrU64::rust_shadowing(&mut stack, vec![], vec![], &mut HashMap::default());
    }

    #[test]
    fn decr_u64_pbt() {
        prop_decr_u64(U32s::new([u32::MAX, 0]));
        prop_decr_u64(U32s::new([0, u32::MAX]));
        prop_decr_u64(U32s::new([u32::MAX, u32::MAX - 1]));
        prop_decr_u64(U32s::new([0, 1]));

        let mut rng = rand::thread_rng();
        for _ in 0..10 {
            prop_decr_u64(U32s::new([0, rng.gen()]));
            prop_decr_u64(U32s::new([rng.gen(), rng.gen()]));
        }
    }

    fn prop_decr_u64(value: U32s<2>) {
        let mut stack = get_init_tvm_stack();
        push_hashable(&mut stack, &value);
        rust_tasm_equivalence_prop::<DecrU64>(&stack, &[], &[], &mut HashMap::default(), 0, None);
    }
}
