use rand::{thread_rng, RngCore};
use twenty_first::shared_math::b_field_element::BFieldElement;

use crate::{
    arithmetic::u32::leading_zeros_u32::LeadingZerosU32,
    get_init_tvm_stack,
    snippet::{DataType, Snippet},
    ExecutionState,
};

#[derive(Clone)]
pub struct LeadingZerosU64;

impl Snippet for LeadingZerosU64 {
    fn entrypoint(&self) -> String {
        "tasm_arithmetic_u64_leading_zeros".to_string()
    }

    fn inputs(&self) -> Vec<String> {
        vec!["value_hi".to_string(), "value_lo".to_string()]
    }

    fn outputs(&self) -> Vec<String> {
        vec!["leading zeros in value".to_string()]
    }

    fn input_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::U64]
    }

    fn output_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::U32]
    }

    fn stack_diff(&self) -> isize {
        -1
    }

    fn function_body(&self, library: &mut crate::library::Library) -> String {
        let leading_zeros_u32 = library.import(Box::new(LeadingZerosU32));
        let entrypoint = self.entrypoint();
        format!(
            "
                // BEFORE: _ value_hi value_lo
                {entrypoint}:
                    swap 1
                    call {leading_zeros_u32}
                    // _ value_lo leading_zeros_value_hi

                    dup 0
                    push 32
                    eq
                    skiz
                        call {entrypoint}_hi_was_zero

                    // _ temp leading_zeros

                    swap 1
                    pop
                    return

                    {entrypoint}_hi_was_zero:
                    // _ value_lo 32

                    swap 1
                    call {leading_zeros_u32}
                    // _ 32 leading_zeros_value_lo

                    dup 1
                    add
                    // _ 32 leading_zeros
                    return
"
        )
    }

    fn crash_conditions() -> Vec<String> {
        vec!["Inputs are not u32".to_owned()]
    }

    fn gen_input_states(&self) -> Vec<crate::ExecutionState> {
        let mut rng = thread_rng();
        let mut ret = vec![];
        for _ in 0..10 {
            ret.push(prepare_state(rng.next_u64()));
        }

        ret
    }

    fn rust_shadowing(
        &self,
        stack: &mut Vec<twenty_first::shared_math::b_field_element::BFieldElement>,
        _std_in: Vec<twenty_first::shared_math::b_field_element::BFieldElement>,
        _secret_in: Vec<twenty_first::shared_math::b_field_element::BFieldElement>,
        _memory: &mut std::collections::HashMap<
            twenty_first::shared_math::b_field_element::BFieldElement,
            twenty_first::shared_math::b_field_element::BFieldElement,
        >,
    ) {
        let value_lo: u32 = stack.pop().unwrap().try_into().unwrap();
        let value_hi: u32 = stack.pop().unwrap().try_into().unwrap();
        let value: u64 = ((value_hi as u64) << 32) + value_lo as u64;

        let value = value.leading_zeros();
        stack.push(BFieldElement::new(value as u64));
    }

    fn common_case_input_state(&self) -> crate::ExecutionState
    where
        Self: Sized,
    {
        prepare_state(1 << 31)
    }

    fn worst_case_input_state(&self) -> crate::ExecutionState
    where
        Self: Sized,
    {
        prepare_state(1 << 62)
    }
}

fn prepare_state(value: u64) -> ExecutionState {
    let value_hi: u32 = (value >> 32) as u32;
    let value_lo: u32 = (value & u32::MAX as u64) as u32;
    let mut stack = get_init_tvm_stack();
    stack.push(BFieldElement::new(value_hi as u64));
    stack.push(BFieldElement::new(value_lo as u64));
    ExecutionState::with_stack(stack)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        snippet_bencher::bench_and_write,
        test_helpers::{rust_tasm_equivalence_prop, rust_tasm_equivalence_prop_new},
    };

    use super::*;

    #[test]
    fn snippet_test() {
        rust_tasm_equivalence_prop_new(LeadingZerosU64);
    }

    #[test]
    fn u32_leading_zeros_benchmark() {
        bench_and_write(LeadingZerosU64);
    }

    #[test]
    fn leading_zeros_u64_simple_test() {
        prop_leading_zeros(1, Some(63));
        prop_leading_zeros(2, Some(62));
        prop_leading_zeros(3, Some(62));
        prop_leading_zeros(4, Some(61));
        prop_leading_zeros(256, Some(55));
        prop_leading_zeros(123, Some(57));
        prop_leading_zeros(0, Some(64));
        prop_leading_zeros(1 << 31, Some(32));
        prop_leading_zeros(1 << 30, Some(33));
        prop_leading_zeros(1 << 29, Some(34));
        prop_leading_zeros(1 << 28, Some(35));
        prop_leading_zeros(u32::MAX as u64, Some(32));
        prop_leading_zeros(1000, Some(54));
        prop_leading_zeros(2000, Some(53));
        prop_leading_zeros(4000, Some(52));
        prop_leading_zeros(4095, Some(52));
        prop_leading_zeros(4096, Some(51));
        prop_leading_zeros(4097, Some(51));
    }

    fn prop_leading_zeros(value: u64, expected: Option<u64>) {
        let mut init_stack = get_init_tvm_stack();
        init_stack.push(BFieldElement::new(value));

        let execution_result = rust_tasm_equivalence_prop(
            LeadingZerosU64,
            &init_stack,
            &[],
            &[],
            &mut HashMap::default(),
            0,
            None,
        );

        let mut final_stack = execution_result.final_stack;
        if let Some(res) = expected {
            let lo: u32 = final_stack.pop().unwrap().try_into().unwrap();
            let hi: u32 = final_stack.pop().unwrap().try_into().unwrap();
            assert_eq!(res, lo as u64 + ((hi as u64) << 32));
        };
    }
}