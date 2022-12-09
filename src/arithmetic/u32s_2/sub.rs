use twenty_first::{amount::u32s::U32s, util_types::algebraic_hasher::Hashable};

use crate::{arithmetic::u32::is_u32::IsU32, snippet_trait::Snippet};

pub struct U32s2Sub();

const SNIPPET_NAME: &str = "u32_2_sub";

impl Snippet for U32s2Sub {
    const STACK_DIFF: isize = -2;

    const NAME: &'static str = SNIPPET_NAME;

    /// Four top elements of stack are assumed to be valid u32s. So to have
    /// a value that's less than 2^32.
    fn get_function() -> String {
        let is_u32_code = IsU32::get_function();
        let is_u32_function_name = IsU32::NAME;
        const TWO_POW_32: &str = "4294967296";

        format!(
            "
            // Import the `is_u32` function
            {is_u32_code}

            {SNIPPET_NAME}_carry:
                push {TWO_POW_32}
                add
                swap2  // -> _ lo_diff hi_l hi_r
                push 1
                add    // -> _ lo_diff hi_l (hi_r + 1)
                swap2  // -> _ (hi_r + 1) hi_l lo_diff
                return

            // Before: _ hi_r lo_r hi_l lo_l
            // After: _ hi_diff lo_diff
            {SNIPPET_NAME}:
                swap1  // -> _ hi_r lo_r lo_l hi_l
                swap2  // -> _ hi_r hi_l lo_l lo_r
                neg
                add    // -> _ hi_r hi_l (lo_l - lo_r)

                dup0
                call {is_u32_function_name}
                push 0
                eq
                skiz
                    call {SNIPPET_NAME}_carry

                swap2  // -> lo_diff hi_l hi_r
                neg
                add    // -> lo_diff (hi_l - hi_r)
                swap1  // -> (hi_l - hi_r) lo_diff

                return
            "
        )
    }

    fn rust_shadowing(
        stack: &mut Vec<twenty_first::shared_math::b_field_element::BFieldElement>,
        _std_in: Vec<twenty_first::shared_math::b_field_element::BFieldElement>,
        _secret_in: Vec<twenty_first::shared_math::b_field_element::BFieldElement>,
    ) {
        // top element on stack
        let a0: u32 = stack.pop().unwrap().try_into().unwrap();
        let b0: u32 = stack.pop().unwrap().try_into().unwrap();
        let ab0 = U32s::<2>::new([a0, b0]);

        // second element on stack
        let a1: u32 = stack.pop().unwrap().try_into().unwrap();
        let b1: u32 = stack.pop().unwrap().try_into().unwrap();
        let ab1 = U32s::<2>::new([a1, b1]);
        let ab0_minus_ab1 = ab0 - ab1;
        let mut res = ab0_minus_ab1.to_sequence();
        for _ in 0..res.len() {
            stack.push(res.pop().unwrap());
        }
    }
}

#[cfg(test)]
mod tests {
    use num::{BigUint, Zero};
    use rand::Rng;
    use twenty_first::shared_math::b_field_element::BFieldElement;

    use crate::get_init_tvm_stack;

    use super::*;

    #[test]
    fn u32s_2_sub_no_overflow() {
        // 256 - 129 = 127
        let expected_end_stack = vec![
            get_init_tvm_stack(),
            vec![BFieldElement::zero(), BFieldElement::new(127)],
        ]
        .concat();
        prop_sub(U32s::from(256), U32s::from(129), Some(&expected_end_stack));
    }

    #[test]
    fn u32s_2_sub_carry() {
        // 2^32 - 1 = ...
        let expected_end_stack = vec![
            get_init_tvm_stack(),
            vec![BFieldElement::zero(), BFieldElement::new(u32::MAX as u64)],
        ]
        .concat();
        prop_sub(
            U32s::from(BigUint::from(1u64 << 32)),
            U32s::from(1),
            Some(&expected_end_stack),
        );
    }

    #[test]
    fn u32s_2_sub_pbt() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let lhs: u64 = rng.gen();
            let rhs: u64 = rng.gen_range(0..=lhs);

            prop_sub(
                U32s::from(BigUint::from(lhs)),
                U32s::from(BigUint::from(rhs)),
                None,
            );
        }
    }

    fn prop_sub(lhs: U32s<2>, rhs: U32s<2>, expected: Option<&[BFieldElement]>) {
        let mut init_stack = get_init_tvm_stack();
        for elem in rhs.to_sequence().into_iter().rev() {
            init_stack.push(elem);
        }
        for elem in lhs.to_sequence().into_iter().rev() {
            init_stack.push(elem);
        }

        let mut tasm_stack = init_stack.clone();
        let execution_result = U32s2Sub::run_tasm(&mut tasm_stack, vec![], vec![]);
        println!(
            "Cycle count for `{SNIPPET_NAME}`: {}",
            execution_result.cycle_count
        );
        println!(
            "Hash table height for `{SNIPPET_NAME}`: {}",
            execution_result.hash_table_height
        );

        let mut rust_stack = init_stack;
        U32s2Sub::rust_shadowing(&mut rust_stack, vec![], vec![]);

        assert_eq!(tasm_stack, rust_stack, "Rust code must match TVM for `sub`");
        if let Some(expected) = expected {
            assert_eq!(
                tasm_stack, expected,
                "TVM must produce expected stack. lhs: {lhs}, rhs: {rhs}"
            );
        }
    }
}
