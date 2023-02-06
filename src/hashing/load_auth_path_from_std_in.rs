use std::collections::HashMap;

use num::Zero;

use twenty_first::shared_math::b_field_element::BFieldElement;
use twenty_first::shared_math::other::random_elements;
use twenty_first::shared_math::rescue_prime_digest::Digest;
use twenty_first::shared_math::rescue_prime_digest::DIGEST_LENGTH;

use crate::library::Library;
use crate::list::unsafe_u32::{push::Push, set_length::SetLength};
use crate::mmr::MAX_MMR_HEIGHT;
use crate::rust_shadowing_helper_functions;
use crate::snippet::DataType;
use crate::snippet::Snippet;
use crate::{get_init_tvm_stack, ExecutionState};

#[derive(Clone)]
pub struct LoadAuthPathFromStdIn;

impl Snippet for LoadAuthPathFromStdIn {
    fn inputs() -> Vec<&'static str> {
        vec![]
    }

    fn outputs() -> Vec<&'static str> {
        vec!["auth_path_pointer"]
    }

    fn input_types(&self) -> Vec<crate::snippet::DataType> {
        vec![]
    }

    fn output_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::List(Box::new(DataType::Digest))]
    }

    fn crash_conditions() -> Vec<&'static str> {
        vec!["Not enough elements in std input"]
    }

    fn gen_input_states() -> Vec<ExecutionState> {
        let mut init_vm_states: Vec<ExecutionState> = vec![];
        for ap_length in 0..MAX_MMR_HEIGHT {
            let ap_elements: Vec<Digest> = random_elements(ap_length);
            let mut std_in = vec![BFieldElement::new(ap_length as u64)];
            for ap_element in ap_elements.iter() {
                rust_shadowing_helper_functions::write_digest_to_std_in(&mut std_in, *ap_element);
            }
            let init_vm_state = ExecutionState {
                stack: get_init_tvm_stack(),
                std_in,
                secret_in: vec![],
                memory: HashMap::default(),
                words_allocated: 0,
            };
            init_vm_states.push(init_vm_state);
        }

        init_vm_states
    }

    fn stack_diff() -> isize {
        1
    }

    fn entrypoint(&self) -> &'static str {
        "load_auth_path_from_std_in"
    }

    fn function_body(&self, library: &mut Library) -> String {
        let entrypoint = self.entrypoint();

        let read_digest_from_std_in = "read_io\n".repeat(DIGEST_LENGTH);

        let set_length = library.import(Box::new(SetLength(DataType::Digest)));
        let push = library.import(Box::new(Push::<DIGEST_LENGTH>(DataType::Digest)));

        // Allocate 1 word for length indication, and `DIGEST_LENGTH` words per auth path element
        // Warning: Statically allocated list. Will be overwritten at same location by subsequent
        // calls to this function
        let auth_path_pointer = library.kmalloc(DIGEST_LENGTH * MAX_MMR_HEIGHT + 1);

        format!(
            "
                // BEFORE: _
                // AFTER: *auth_path
                {entrypoint}:
                    // Read length of authentication path
                    read_io
                    // _ total_auth_path_length

                    // It shouldn't be necessary to validate that this is a u32 since
                    // the below loop will never finish if it isn't.

                    // Initialize a counter counting how many elements was read
                    push 0
                    // _ total_auth_path_length i

                    // Initialize list where the authentication path is stored
                    push {auth_path_pointer}
                    push 0
                    call {set_length}
                    // _ total_auth_path_length i *auth_path

                    call {entrypoint}_while
                    // _ total_auth_path_length i *auth_path

                    swap2 pop pop
                    // _ *auth_path

                    return

                // Start/end stack: _ total_auth_path_length i *auth_path
                {entrypoint}_while:
                    // Loop condition: end if (total_auth_path_length == i)
                    dup2 dup2 eq skiz return
                    // _ total_auth_path_length i *auth_path

                    dup0
                    // _ total_auth_path_length i *auth_path *auth_path

                    {read_digest_from_std_in}
                    // _ total_auth_path_length i *auth_path *auth_path [digests (ap_element)]

                    call {push}
                    // _ total_auth_path_length i *auth_path

                    // i -> i + 1
                    swap1 push 1 add swap1
                    // _ total_auth_path_length (i + 1) *auth_path

                    recurse
                "
        )
    }

    fn rust_shadowing(
        stack: &mut Vec<BFieldElement>,
        std_in: Vec<BFieldElement>,
        _secret_in: Vec<BFieldElement>,
        memory: &mut HashMap<BFieldElement, BFieldElement>,
    ) {
        let mut std_in_cursor = 0;
        let total_auth_path_length: u32 = std_in[std_in_cursor].value().try_into().unwrap();
        std_in_cursor += 1;

        let auth_path_pointer = BFieldElement::zero();
        rust_shadowing_helper_functions::unsafe_list_new(auth_path_pointer, memory);

        let mut i = 0;
        while i != total_auth_path_length {
            let ap_element = rust_shadowing_helper_functions::read_digest_from_std_in(
                &std_in,
                &mut std_in_cursor,
            );
            rust_shadowing_helper_functions::unsafe_list_push(
                auth_path_pointer,
                ap_element.values(),
                memory,
            );

            i += 1;
        }

        stack.push(auth_path_pointer);
    }
}

#[cfg(test)]
mod load_auth_path_from_std_in_tests {
    use super::*;
    use crate::snippet_bencher::bench_and_write;
    use crate::test_helpers::rust_tasm_equivalence_prop_new;

    #[test]
    fn load_auth_path_from_std_in_test() {
        rust_tasm_equivalence_prop_new::<LoadAuthPathFromStdIn>(LoadAuthPathFromStdIn);
    }

    #[test]
    fn load_auth_path_from_std_in_benchmark() {
        bench_and_write::<LoadAuthPathFromStdIn>(LoadAuthPathFromStdIn);
    }
}
