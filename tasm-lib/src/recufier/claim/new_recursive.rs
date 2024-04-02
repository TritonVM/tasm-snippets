use num::Zero;
use triton_vm::instruction::LabelledInstruction;
use triton_vm::op_stack::NUM_OP_STACK_REGISTERS;
use triton_vm::prelude::*;
use triton_vm::twenty_first::shared_math::b_field_element::BFieldElement;
use triton_vm::twenty_first::shared_math::bfield_codec::BFieldCodec;

use crate::data_type::DataType;
use crate::io::InputSource;
use crate::library::Library;
use crate::memory::write_words_to_memory_pop_pointer;
use crate::recufier::claim::shared::claim_type;
use crate::traits::basic_snippet::BasicSnippet;
use crate::{Digest, DIGEST_LENGTH};

/// Return a pointer to a claim representing the verification of a proof of the program's own
/// execution. Must be called with an empty stack, as the program digest is read from the bottom
/// of the stack.
#[derive(Debug, Clone)]
pub struct NewRecursive {
    input_size: usize,
    output_size: usize,
}

impl BasicSnippet for NewRecursive {
    fn inputs(&self) -> Vec<(DataType, String)> {
        vec![]
    }

    fn outputs(&self) -> Vec<(DataType, String)> {
        vec![(DataType::StructRef(claim_type()), "*claim".to_owned())]
    }

    fn entrypoint(&self) -> String {
        "tasm_recufier_claim_new_recursive".to_owned()
    }

    fn code(&self, library: &mut Library) -> Vec<LabelledInstruction> {
        let entrypoint = self.entrypoint();
        let claim_size = Claim {
            program_digest: Digest::default(),
            input: vec![BFieldElement::zero(); self.input_size],
            output: vec![BFieldElement::zero(); self.output_size],
        }
        .encode()
        .len();
        let claim_pointer = library.kmalloc(claim_size.try_into().unwrap());
        const METADATA_SIZE_FOR_FIELD_WITH_VEC_VALUE: usize = 2;
        let output_field_pointer = claim_pointer;
        let output_field_size: u32 = (1 + self.output_size).try_into().unwrap();
        let input_field_pointer = output_field_pointer + bfe!(output_field_size + 1);
        let input_field_size: u32 = (1 + self.input_size).try_into().unwrap();
        let digest_field_pointer = input_field_pointer + bfe!(input_field_size + 1);

        let read_output_value = InputSource::SecretIn.read_words(self.output_size);
        let write_output_value_and_metadata = write_words_to_memory_pop_pointer(
            self.output_size + METADATA_SIZE_FOR_FIELD_WITH_VEC_VALUE,
        );
        let read_input_value = InputSource::SecretIn.read_words(self.input_size);
        let write_input_value_and_metadata = write_words_to_memory_pop_pointer(
            self.input_size + METADATA_SIZE_FOR_FIELD_WITH_VEC_VALUE,
        );
        let dup_own_program_digest =
            vec![triton_asm!(dup {NUM_OP_STACK_REGISTERS - 1}); DIGEST_LENGTH].concat();
        let write_digest_to_memory = write_words_to_memory_pop_pointer(DIGEST_LENGTH);

        triton_asm!(
            {entrypoint}:
                // _


                // Write output and its length indicator for the field and the length indicator for
                // the list to memory
                {&read_output_value}
                push {self.output_size}
                push {output_field_size}

                push {output_field_pointer}
                // _ [output] output_size output_field_size *output_field_size

                {&write_output_value_and_metadata}
                // _

                {&read_input_value}
                push {self.input_size}
                push {input_field_size}
                // _ [input] input_size input_field_size

                push {input_field_pointer}
                // _ [input] input_size input_field_size *input_field_size

                {&write_input_value_and_metadata}
                // _

                // Write own digest to claim. It is assumed that own program digest occupies stack
                // words stack[15..=11]

                {&dup_own_program_digest}
                // _ d4 d3 d2 d1 d0

                push {digest_field_pointer}
                {&write_digest_to_memory}
                // _

                push {claim_pointer}
                // _ *claim

                return
        )
    }
}

#[cfg(test)]
pub mod tests {
    use std::collections::{HashMap, VecDeque};

    use itertools::Itertools;
    use test_strategy::proptest;

    use crate::memory::encode_to_memory;
    use crate::snippet_bencher::BenchmarkCase;
    use crate::traits::procedure::{Procedure, ProcedureInitialState, ShadowedProcedure};
    use crate::traits::rust_shadow::RustShadow;
    use crate::VmHasher;

    use super::*;

    #[test]
    fn new_recursive_claim_small_params_pbt() {
        ShadowedProcedure::new(NewRecursive {
            input_size: DIGEST_LENGTH,
            output_size: 0,
        })
        .test()
    }

    #[proptest(cases = 10)]
    fn new_recursive_claim_pbt_pbt(
        #[strategy(0_usize..200)] output_size: usize,
        #[strategy(0_usize..200)] input_size: usize,
    ) {
        ShadowedProcedure::new(NewRecursive {
            input_size,
            output_size,
        })
        .test()
    }

    impl Procedure for NewRecursive {
        fn rust_shadow(
            &self,
            stack: &mut Vec<BFieldElement>,
            memory: &mut HashMap<BFieldElement, BFieldElement>,
            nondeterminism: &NonDeterminism<BFieldElement>,
            _public_input: &[BFieldElement],
            _sponge: &mut Option<VmHasher>,
        ) -> Vec<BFieldElement> {
            println!(
                "nondeterminism.individual_tokens: {}",
                nondeterminism.individual_tokens.iter().join(",")
            );
            let mut individual_tokens: VecDeque<BFieldElement> =
                nondeterminism.individual_tokens.to_owned().into();

            let mut output = vec![];
            for _ in 0..self.output_size {
                output.push(individual_tokens.pop_front().unwrap());
            }
            output.reverse();

            let mut input = vec![];
            for _ in 0..self.input_size {
                input.push(individual_tokens.pop_front().unwrap());
            }
            input.reverse();

            let program_digest = Digest::new([stack[4], stack[3], stack[2], stack[1], stack[0]]);

            let claim = Claim {
                program_digest,
                input,
                output,
            };

            // Statically allocated memory starts at -2 and grows downward. So a value of size 1
            // will be assigned to address `-2` if no other static memory allocations have occurred
            // before it.
            let claim_pointer = bfe!(-(claim.encode().len() as i32) - 1);

            encode_to_memory(memory, claim_pointer, claim);

            stack.push(claim_pointer);

            vec![]
        }

        fn pseudorandom_initial_state(
            &self,
            _seed: [u8; 32],
            _bench_case: Option<BenchmarkCase>,
        ) -> ProcedureInitialState {
            let output = (0..self.input_size)
                .map(|x| BFieldElement::new(x as u64))
                .rev()
                .collect_vec();
            let input = (100..(100 + self.output_size))
                .map(|x| BFieldElement::new(x as u64))
                .rev()
                .collect_vec();

            let nondeterminism = NonDeterminism {
                individual_tokens: [output, input].concat(),
                ..Default::default()
            };
            ProcedureInitialState {
                stack: self.init_stack_for_isolated_run(),
                nondeterminism,
                public_input: vec![],
                sponge: None,
            }
        }
    }
}
