use std::collections::HashMap;

use rand::RngCore;
use twenty_first::shared_math::b_field_element::BFieldElement;

use crate::pseudo::lsb::Lsb;
use crate::snippet::{DataType, Snippet};
use crate::snippet_state::SnippetState;
use crate::{get_init_tvm_stack, push_encodable, ExecutionState};

#[derive(Clone, Debug)]
pub struct U32IsOdd;

impl Snippet for U32IsOdd {
    fn inputs(&self) -> Vec<String> {
        vec!["value".to_string()]
    }

    fn outputs(&self) -> Vec<String> {
        vec!["value % 2".to_string()]
    }

    fn input_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::U32]
    }

    fn output_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::Bool]
    }

    fn crash_conditions() -> Vec<String> {
        vec!["if `value` is not a u32".to_string()]
    }

    fn gen_input_states(&self) -> Vec<ExecutionState> {
        let n: u32 = rand::thread_rng().next_u32();

        let mut even_stack = get_init_tvm_stack();
        let even_value = n - (n & 1);
        push_encodable(&mut even_stack, &even_value);

        let mut odd_stack = get_init_tvm_stack();
        let odd_value = n | 1;
        push_encodable(&mut odd_stack, &odd_value);

        vec![
            ExecutionState::with_stack(even_stack),
            ExecutionState::with_stack(odd_stack),
        ]
    }

    fn stack_diff(&self) -> isize {
        // pops a u32 from the stack and pushes a bool
        0
    }

    fn entrypoint(&self) -> String {
        "tasm_arithmetic_u32_is_odd".to_string()
    }

    fn function_body(&self, library: &mut SnippetState) -> String {
        let entrypoint = self.entrypoint();
        let lsb = library.import(Box::new(Lsb));
        format!(
            "
                // BEFORE: _ value
                // AFTER: _ (value % 2)
                {entrypoint}:
                    [{lsb}]
                    swap 1
                    pop
                    return
                "
        )
    }

    fn rust_shadowing(
        &self,
        stack: &mut Vec<BFieldElement>,
        _std_in: Vec<BFieldElement>,
        _secret_in: Vec<BFieldElement>,
        _memory: &mut HashMap<BFieldElement, BFieldElement>,
    ) {
        let value: u64 = stack.pop().unwrap().value();
        stack.push(BFieldElement::new(value % 2));
    }

    fn common_case_input_state(&self) -> ExecutionState
    where
        Self: Sized,
    {
        ExecutionState::with_stack(
            vec![get_init_tvm_stack(), vec![BFieldElement::new(1 << 16)]].concat(),
        )
    }

    fn worst_case_input_state(&self) -> ExecutionState
    where
        Self: Sized,
    {
        ExecutionState::with_stack(
            vec![
                get_init_tvm_stack(),
                vec![BFieldElement::new((1 << 32) - 1)],
            ]
            .concat(),
        )
    }
}

#[cfg(test)]
mod u32_is_odd_tests {
    use rand::{thread_rng, RngCore};

    use crate::get_init_tvm_stack;
    use crate::snippet_bencher::bench_and_write;
    use crate::test_helpers::{rust_tasm_equivalence_prop, rust_tasm_equivalence_prop_new};

    use super::*;

    #[test]
    fn is_odd_u32_test() {
        rust_tasm_equivalence_prop_new(&U32IsOdd, true);
    }

    #[test]
    fn is_odd_u32_benchmark() {
        bench_and_write(U32IsOdd);
    }

    #[test]
    fn u32_is_odd_test() {
        prop_is_odd(0);
        prop_is_odd(1);
        prop_is_odd(2);
        prop_is_odd(3);
        prop_is_odd(4);
        prop_is_odd(5);
        prop_is_odd(6);
        prop_is_odd(7);
        prop_is_odd(8);
        prop_is_odd(u32::MAX);

        let mut rng = thread_rng();
        for _ in 0..100 {
            let value = rng.next_u32();
            prop_is_odd(value);
        }
    }

    fn prop_is_odd(value: u32) {
        let mut init_stack = get_init_tvm_stack();
        init_stack.push(BFieldElement::new(value as u64));
        let mut expected_stack = get_init_tvm_stack();
        expected_stack.push(BFieldElement::new((value % 2) as u64));

        let _execution_result = rust_tasm_equivalence_prop(
            &U32IsOdd,
            &init_stack,
            &[],
            &[],
            &mut HashMap::default(),
            0,
            Some(&expected_stack),
        );
    }
}
