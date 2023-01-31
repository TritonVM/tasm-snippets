use std::collections::HashMap;

use rand::Rng;
use twenty_first::shared_math::b_field_element::BFieldElement;

use crate::snippet::{DataType, Snippet};
use crate::{get_init_tvm_stack, ExecutionState};

#[derive(Clone)]
pub struct Neg;

impl Snippet for Neg {
    fn inputs() -> Vec<&'static str> {
        vec!["value"]
    }

    fn outputs() -> Vec<&'static str> {
        vec!["-value"]
    }

    fn input_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::BFE]
    }

    fn output_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::BFE]
    }

    fn crash_conditions() -> Vec<&'static str> {
        vec![]
    }

    fn gen_input_states() -> Vec<ExecutionState> {
        let mut rng = rand::thread_rng();
        let mut stack = get_init_tvm_stack();
        stack.push(rng.gen());
        vec![ExecutionState::with_stack(stack)]
    }

    fn stack_diff() -> isize {
        0
    }

    fn entrypoint() -> &'static str {
        "neg"
    }

    fn function_body(_library: &mut crate::library::Library) -> String {
        let entrypoint = Self::entrypoint();

        format!(
            "
            {entrypoint}:
                push -1
                mul
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
        let elem = stack.pop().unwrap();
        stack.push(-elem);
    }
}
