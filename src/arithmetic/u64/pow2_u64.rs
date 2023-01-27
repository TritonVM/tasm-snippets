use std::collections::HashMap;

use num::One;
use twenty_first::amount::u32s::U32s;
use twenty_first::shared_math::b_field_element::BFieldElement;
use twenty_first::util_types::algebraic_hasher::Hashable;

use crate::library::Library;
use crate::snippet::Snippet;
use crate::{get_init_tvm_stack, push_hashable, ExecutionState};

/// Consumes top element which is interpreted as exponent. Pushes a
/// U32<2> to the top of the stack. So grows the stack by 1.
pub struct Pow2U64;

impl Snippet for Pow2U64 {
    fn inputs() -> Vec<&'static str> {
        vec!["i"]
    }

    fn outputs() -> Vec<&'static str> {
        vec!["(2^i)_hi", "(2^i)_lo"]
    }

    fn crash_conditions() -> Vec<&'static str> {
        vec![]
    }

    fn gen_input_states() -> Vec<ExecutionState> {
        (0..64)
            .map(|i: u32| {
                let mut stack = get_init_tvm_stack();
                push_hashable(&mut stack, &i);
                ExecutionState::with_stack(stack)
            })
            .collect()
    }

    fn stack_diff() -> isize {
        1
    }

    fn entrypoint() -> &'static str {
        "pow2_u64"
    }

    fn function_body(_library: &mut Library) -> String {
        let entrypoint = Self::entrypoint();

        format!(
            "{entrypoint}:
                push 2
                pow
                split
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
        // Find exponent
        let mut exponent: u32 = stack.pop().unwrap().try_into().unwrap();
        let mut res = U32s::<2>::one();

        while exponent > 0 {
            res.mul_two();
            exponent -= 1;
        }

        let mut res = res.to_sequence();
        for _ in 0..res.len() {
            stack.push(res.pop().unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use num::One;

    use crate::get_init_tvm_stack;
    use crate::snippet_bencher::bench_and_write;
    use crate::test_helpers::{rust_tasm_equivalence_prop, rust_tasm_equivalence_prop_new};

    use super::*;

    #[test]
    fn pow2_static_test() {
        rust_tasm_equivalence_prop_new::<Pow2U64>();
    }

    #[test]
    fn pow2_static_benchmark() {
        bench_and_write::<Pow2U64>();
    }

    fn prop_exp_static(exponent: u8) {
        let mut init_stack = get_init_tvm_stack();
        init_stack.push(BFieldElement::new(exponent as u64));

        let expected = None;
        let mut execution_result = rust_tasm_equivalence_prop::<Pow2U64>(
            &init_stack,
            &[],
            &[],
            &mut HashMap::default(),
            0,
            expected,
        );

        let a = execution_result.final_stack.pop().unwrap().value();
        assert!(a < u32::MAX as u64);
        let b = execution_result.final_stack.pop().unwrap().value();
        assert!(b < u32::MAX as u64);
        let actual_res = U32s::<2>::new([a as u32, b as u32]);
        let mut expected_res = U32s::<2>::one();
        for _ in 0..exponent {
            expected_res.mul_two();
        }

        assert_eq!(expected_res, actual_res);
    }

    #[test]
    fn all_exponents_static() {
        for i in 0..64 {
            prop_exp_static(i);
        }
    }
}
