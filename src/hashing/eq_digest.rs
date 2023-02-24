use std::collections::HashMap;

use rand::Rng;
use twenty_first::shared_math::b_field_element::BFieldElement;
use twenty_first::shared_math::rescue_prime_digest::Digest;

use crate::library::Library;
use crate::snippet::{DataType, Snippet};
use crate::{get_init_tvm_stack, push_hashable, ExecutionState};

#[derive(Clone)]
pub struct EqDigest;

impl Snippet for EqDigest {
    fn inputs(&self) -> Vec<String> {
        vec![
            "b4".to_string(),
            "b3".to_string(),
            "b2".to_string(),
            "b1".to_string(),
            "b0".to_string(),
            "a4".to_string(),
            "a3".to_string(),
            "a2".to_string(),
            "a1".to_string(),
            "a0".to_string(),
        ]
    }

    fn outputs(&self) -> Vec<String> {
        vec!["(a3 = b3)·(a2 = b2)·(a1 = b1)·(a4 = b4)·(b0 = a0)".to_string()]
    }

    fn input_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::Digest, DataType::Digest]
    }

    fn output_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::Bool]
    }

    fn crash_conditions() -> Vec<String> {
        vec![]
    }

    fn gen_input_states(&self) -> Vec<ExecutionState> {
        let mut rng = rand::thread_rng();
        let digest_a: Digest = rng.gen();
        let digest_b: Digest = rng.gen();

        let mut stack = get_init_tvm_stack();
        push_hashable(&mut stack, &digest_b);
        push_hashable(&mut stack, &digest_a);

        vec![ExecutionState::with_stack(stack)]
    }

    fn stack_diff(&self) -> isize {
        -9
    }

    fn entrypoint(&self) -> String {
        "tasm_hashing_eq_digest".to_string()
    }

    fn function_body(&self, _library: &mut Library) -> String {
        let entrypoint = self.entrypoint();
        format!(
            "
            // Before: _ b4 b3 b2 b1 b0 a4 a3 a2 a1 a0
            // After: _ (a3 = b3)·(a2 = b2)·(a1 = b1)·(a4 = b4)·(b0 = a0)
            {entrypoint}:
                swap6  // _ b4 b3 b2 a0 b0 a4 a3 a2 a1 b1
                eq     // _ b4 b3 b2 a0 b0 a4 a3 a2 (a1 = b1)
                swap6  // _ b4 b3 (a1 = b1) a0 b0 a4 a3 a2 b2
                eq     // _ b4 b3 (a1 = b1) a0 b0 a4 a3 (a2 = b2)
                swap6  // _ b4 (a2 = b2) (a1 = b1) a0 b0 a4 a3 b3
                eq     // _ b4 (a2 = b2) (a1 = b1) a0 b0 a4 (a3 = b3)
                swap6  // _ (a3 = b3) (a2 = b2) (a1 = b1) a0 b0 a4 b4
                eq     // _ (a3 = b3) (a2 = b2) (a1 = b1) a0 b0 (a4 = b4)
                swap2  // _ (a3 = b3) (a2 = b2) (a1 = b1) (a4 = b4) b0 a0
                eq     // _ (a3 = b3) (a2 = b2) (a1 = b1) (a4 = b4) (b0 = a0)
                
                mul
                mul
                mul
                mul    // (a3 = b3)·(a2 = b2)·(a1 = b1)·(a4 = b4)·(b0 = a0)

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
        let digest_a = Digest::new([
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
        ]);

        let digest_b = Digest::new([
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
        ]);

        stack.push(BFieldElement::new((digest_a == digest_b) as u64));
    }

    fn common_case_input_state(&self) -> ExecutionState
    where
        Self: Sized,
    {
        todo!()
    }

    fn worst_case_input_state(&self) -> ExecutionState
    where
        Self: Sized,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use crate::snippet_bencher::bench_and_write;
    use crate::test_helpers::rust_tasm_equivalence_prop_new;

    use super::*;

    #[test]
    fn swap_digest_test() {
        rust_tasm_equivalence_prop_new::<EqDigest>(EqDigest);
    }

    #[test]
    fn swap_digest_benchmark() {
        bench_and_write::<EqDigest>(EqDigest);
    }
}
