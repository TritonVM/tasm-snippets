use itertools::Itertools;
use num_traits::One;
use rand::{thread_rng, RngCore};
use triton_vm::BFieldElement;
use twenty_first::util_types::{
    algebraic_hasher::AlgebraicHasher, merkle_tree::CpuParallel, merkle_tree_maker::MerkleTreeMaker,
};

use crate::{
    hashing::hash_varlen::HashVarlen,
    list::unsafe_u32::{
        get::UnsafeGet, new::UnsafeNew, set::UnsafeSet, set_length::UnsafeSetLength,
    },
    neptune::transaction::transaction_kernel::random_transaction_kernel_encoding,
    rust_shadowing_helper_functions,
    snippet::{DataType, Snippet},
    structure::get_field_with_size::GetFieldWithSize,
    Digest, VmHasher, DIGEST_LENGTH,
};

use super::transaction_kernel::*;

/// Computes the mast hash of a transaction kernel object
#[derive(Debug, Clone)]
pub struct TransactionKernelMastHash;

impl Snippet for TransactionKernelMastHash {
    fn entrypoint(&self) -> String {
        "tasm_neptune_transaction_transaction_kernel_mast_hash".to_string()
    }

    fn inputs(&self) -> Vec<String> {
        vec!["*addr".to_string()]
    }

    fn input_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::VoidPointer]
    }

    fn output_types(&self) -> Vec<crate::snippet::DataType> {
        vec![DataType::Digest]
    }

    fn outputs(&self) -> Vec<String> {
        vec![
            "d4".to_string(),
            "d3".to_string(),
            "d2".to_string(),
            "d1".to_string(),
            "d0".to_string(),
        ]
    }

    fn stack_diff(&self) -> isize {
        4
    }

    fn function_code(&self, library: &mut crate::snippet_state::SnippetState) -> String {
        let entrypoint = self.entrypoint();
        let new_list = library.import(Box::new(UnsafeNew(DataType::Digest)));
        let get_element = library.import(Box::new(UnsafeGet(DataType::Digest)));
        let set_element = library.import(Box::new(UnsafeSet(DataType::Digest)));
        let set_length = library.import(Box::new(UnsafeSetLength(DataType::Digest)));

        let get_field_with_size = library.import(Box::new(GetFieldWithSize));

        let hash_varlen = library.import(Box::new(HashVarlen));

        format!(
            "
        // BEFORE: _ *kernel
        // AFTER: _ d4 d3 d2 d1 d0
        {entrypoint}:
            // allocate new list of 16 digests
            push 16                      // _ *kernel 16
            dup 0                        // _ *kernel 16 16
            call {new_list}              // _ *kernel 16 *list
            swap 1                       // _ *kernel *list 16
            call {set_length}            // _ *kernel *list

            // populate list[8] with inputs digest
            dup 1                       // _ *kernel *list *kernel
            push 0
            call {get_field_with_size}  // _ *kernel *list *inputs *inputs_size
            call {hash_varlen}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 8                // _ *kernel *list d4 d3 d2 d1 d0 *list 8
            call {set_element}          // _ *kernel *list

            // populate list[9] with outputs digest
            dup 1                       // _ *kernel *list *kernel
            push 1
            call {get_field_with_size}  // _ *kernel *list *outputs *outputs_size
            call {hash_varlen}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 9                // _ *kernel *list d4 d3 d2 d1 d0 *list 9
            call {set_element}          // _ *kernel *list

            // populate list[10] with pubscript_hashes_and_inputs digest
            dup 1                       // _ *kernel *list *kernel
            push 2
            call {get_field_with_size}  // _ *kernel *list *pubscript_hashes_and_inputs *pubscript_hashes_and_inputs_size_size
            call {hash_varlen}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 10               // _ *kernel *list d4 d3 d2 d1 d0 *list 10
            call {set_element}          // _ *kernel *list

            // populate list[11] with fee digest
            dup 1                       // _ *kernel *list *kernel
            push 3
            call {get_field_with_size}  // _ *kernel *list *fee *fee_size
            call {hash_varlen}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 11               // _ *kernel *list d4 d3 d2 d1 d0 *list 11
            call {set_element}          // _ *kernel *list

            // populate list[12] with coinbase digest
            dup 1                       // _ *kernel *list *kernel
            push 4
            call {get_field_with_size}  // _ *kernel *list *coinbase *coinbase_size
            call {hash_varlen}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 12               // _ *kernel *list d4 d3 d2 d1 d0 *list 12
            call {set_element}          // _ *kernel *list

            // populate list[13] with timestamp digest
            dup 1                       // _ *kernel *list *kernel
            push 5
            call {get_field_with_size}  // _ *kernel *list *timestamp *timestamp_size
            call {hash_varlen}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 13               // _ *kernel *list d4 d3 d2 d1 d0 *list 13
            call {set_element}          // _ *kernel *list

            // populate list[14] with mutator set hash digest
            dup 1                       // _ *kernel *list *kernel
            push 6
            call {get_field_with_size}  // _ *kernel *list *mutator_set_hash *mutator_set_hash_size
            call {hash_varlen}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 14               // _ *kernel *list d4 d3 d2 d1 d0 *list 14
            call {set_element}          // _ *kernel *list

            // populate list[15] with default digest
            push 0 push 0 push 0 push 0 push 0
            dup 5 push 15               // _ *kernel *list d4 d3 d2 d1 d0 *list 15
            call {set_element}          // _ *kernel *list

            // hash 14||15 and store in 7
            dup 0 push 15               // _ *kernel *list *list 15
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 14               // _ *kernel *list d4 d3 d2 d1 d0 *list 14
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0 e4 e3 e2 e1 e0
            hash                        // _ *kernel *list f4 f3 f2 f1 f0 0 0 0 0 0
            pop pop pop pop pop         // _ *kernel *list f4 f3 f2 f1 f0
            dup 5 push 7                // _ *kernel *list f4 f3 f2 f1 f0 *list 7
            call {set_element}

            // hash 12||13 and store in 6
            dup 0 push 13               // _ *kernel *list *list 13
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 12               // _ *kernel *list d4 d3 d2 d1 d0 *list 12
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0 e4 e3 e2 e1 e0
            hash                        // _ *kernel *list f4 f3 f2 f1 f0 0 0 0 0 0
            pop pop pop pop pop         // _ *kernel *list f4 f3 f2 f1 f0
            dup 5 push 6                // _ *kernel *list f4 f3 f2 f1 f0 *list 6
            call {set_element}

            // hash 10||11 and store in 5
            dup 0 push 11               // _ *kernel *list *list 11
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 10               // _ *kernel *list d4 d3 d2 d1 d0 *list 10
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0 e4 e3 e2 e1 e0
            hash                        // _ *kernel *list f4 f3 f2 f1 f0 0 0 0 0 0
            pop pop pop pop pop         // _ *kernel *list f4 f3 f2 f1 f0
            dup 5 push 5                // _ *kernel *list f4 f3 f2 f1 f0 *list 5
            call {set_element}

            // hash 8||9 and store in 4
            dup 0 push 9                // _ *kernel *list *list 9
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 8                // _ *kernel *list d4 d3 d2 d1 d0 *list 8
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0 e4 e3 e2 e1 e0
            hash                        // _ *kernel *list f4 f3 f2 f1 f0 0 0 0 0 0
            pop pop pop pop pop         // _ *kernel *list f4 f3 f2 f1 f0
            dup 5 push 4                // _ *kernel *list f4 f3 f2 f1 f0 *list 4
            call {set_element}

            // hash 6||7 and store in 3
            dup 0 push 7                // _ *kernel *list *list 7
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 6                // _ *kernel *list d4 d3 d2 d1 d0 *list 6
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0 e4 e3 e2 e1 e0
            hash                        // _ *kernel *list f4 f3 f2 f1 f0 0 0 0 0 0
            pop pop pop pop pop         // _ *kernel *list f4 f3 f2 f1 f0
            dup 5 push 3                // _ *kernel *list f4 f3 f2 f1 f0 *list 3
            call {set_element}

            // hash 4||5 and store in 2
            dup 0 push 5                // _ *kernel *list *list 5
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 4                // _ *kernel *list d4 d3 d2 d1 d0 *list 4
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0 e4 e3 e2 e1 e0
            hash                        // _ *kernel *list f4 f3 f2 f1 f0 0 0 0 0 0
            pop pop pop pop pop         // _ *kernel *list f4 f3 f2 f1 f0
            dup 5 push 2                // _ *kernel *list f4 f3 f2 f1 f0 *list 2
            call {set_element}

            // hash 2||3 and store in 1
            dup 0 push 3                // _ *kernel *list *list 3
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0
            dup 5 push 2                // _ *kernel *list d4 d3 d2 d1 d0 *list 2
            call {get_element}          // _ *kernel *list d4 d3 d2 d1 d0 e4 e3 e2 e1 e0
            hash                        // _ *kernel *list f4 f3 f2 f1 f0 0 0 0 0 0
            pop pop pop pop pop         // _ *kernel *list f4 f3 f2 f1 f0
            dup 5 push 1                // _ *kernel *list f4 f3 f2 f1 f0 *list 1

            call {set_element}

            // return list[1]
            swap 1                      // _ *list *kernel
            pop
            push 1 // _ *list 1
            call {get_element}          // _ d4 d3 d2 d1 d0

            return
            "
        )
    }

    fn crash_conditions(&self) -> Vec<String> {
        vec!["Transaction kernel is improperly formatted in memory.".to_string()]
    }

    fn gen_input_states(&self) -> Vec<crate::ExecutionState> {
        vec![
            input_state_with_kernel_in_memory(
                BFieldElement::new(thread_rng().next_u64() % (1 << 20)),
                &random_transaction_kernel_encoding(),
            ),
            input_state_with_kernel_in_memory(
                BFieldElement::new(thread_rng().next_u64() % (1 << 20)),
                &random_transaction_kernel_encoding(),
            ),
            input_state_with_kernel_in_memory(
                BFieldElement::new(thread_rng().next_u64() % (1 << 20)),
                &random_transaction_kernel_encoding(),
            ),
            input_state_with_kernel_in_memory(
                BFieldElement::new(thread_rng().next_u64() % (1 << 20)),
                &random_transaction_kernel_encoding(),
            ),
            input_state_with_kernel_in_memory(
                BFieldElement::new(thread_rng().next_u64() % (1 << 20)),
                &example_transaction_kernel_encoded(),
            ),
        ]
    }

    fn common_case_input_state(&self) -> crate::ExecutionState {
        let mut seed: [u8; 32] = [0u8; 32];
        seed[0] = 0xba;
        seed[1] = 0xdd;
        seed[2] = 0xbe;
        seed[3] = 0xef;
        input_state_with_kernel_in_memory(
            BFieldElement::new(1),
            &pseudorandom_transaction_kernel_encoding(seed, 360, 2, 500),
        )
    }

    fn worst_case_input_state(&self) -> crate::ExecutionState {
        let mut seed: [u8; 32] = [0u8; 32];
        seed[0] = 0xba;
        seed[1] = 0xdd;
        seed[2] = 0xbe;
        seed[3] = 0xef;
        input_state_with_kernel_in_memory(
            BFieldElement::new(1),
            &pseudorandom_transaction_kernel_encoding(seed, 3600, 20, 5000),
        )
    }

    fn rust_shadowing(
        &self,
        stack: &mut Vec<triton_vm::BFieldElement>,
        _std_in: Vec<triton_vm::BFieldElement>,
        _secret_in: Vec<triton_vm::BFieldElement>,
        memory: &mut std::collections::HashMap<triton_vm::BFieldElement, triton_vm::BFieldElement>,
    ) {
        // read address
        let mut address = stack.pop().unwrap();

        // inputs
        let inputs_size = memory.get(&address).unwrap().value() as usize;
        let inputs_encoded = (0..inputs_size)
            .map(|i| {
                *memory
                    .get(&(address + BFieldElement::new(1 + i as u64)))
                    .unwrap()
            })
            .collect_vec();
        let inputs_hash = VmHasher::hash_varlen(&inputs_encoded);
        address += BFieldElement::one() + BFieldElement::new(inputs_size as u64);

        // outputs
        let outputs_size = memory.get(&address).unwrap().value() as usize;
        let outputs_encoded = (0..outputs_size)
            .map(|i| {
                *memory
                    .get(&(address + BFieldElement::new(1 + i as u64)))
                    .unwrap()
            })
            .collect_vec();
        let outputs_hash = VmHasher::hash_varlen(&outputs_encoded);
        address += BFieldElement::one() + BFieldElement::new(outputs_size as u64);

        // pubscript_hashes_and_inputs
        let pubscript_hashes_and_inputs_size = memory.get(&address).unwrap().value() as usize;
        let pubscript_hashes_and_inputs_encoded = (0..pubscript_hashes_and_inputs_size)
            .map(|i| {
                *memory
                    .get(&(address + BFieldElement::new(1 + i as u64)))
                    .unwrap()
            })
            .collect_vec();
        let pubscript_hashes_and_inputs_hash =
            VmHasher::hash_varlen(&pubscript_hashes_and_inputs_encoded);
        address +=
            BFieldElement::one() + BFieldElement::new(pubscript_hashes_and_inputs_size as u64);

        // fee
        let fee_size = memory.get(&address).unwrap().value() as usize;
        let fee_encoded = (0..fee_size)
            .map(|i| {
                *memory
                    .get(&(address + BFieldElement::new(1 + i as u64)))
                    .unwrap()
            })
            .collect_vec();
        let fee_hash = VmHasher::hash_varlen(&fee_encoded);
        address += BFieldElement::one() + BFieldElement::new(fee_size as u64);

        // coinbase
        let coinbase_size = memory.get(&address).unwrap().value() as usize;
        let coinbase_encoded = (0..coinbase_size)
            .map(|i| {
                *memory
                    .get(&(address + BFieldElement::new(1 + i as u64)))
                    .unwrap()
            })
            .collect_vec();
        let coinbase_hash = VmHasher::hash_varlen(&coinbase_encoded);
        address += BFieldElement::one() + BFieldElement::new(coinbase_size as u64);

        // timestamp
        let timestamp_size = memory.get(&address).unwrap().value() as usize;
        assert_eq!(timestamp_size, 1);
        let timestamp_encoded = (0..timestamp_size)
            .map(|i| {
                *memory
                    .get(&(address + BFieldElement::new(1 + i as u64)))
                    .unwrap()
            })
            .collect_vec();
        let timestamp_hash = VmHasher::hash_varlen(&timestamp_encoded);
        address += BFieldElement::one() + BFieldElement::new(timestamp_size as u64);

        // mutator_set_hash
        let mutator_set_hash_size = memory.get(&address).unwrap().value() as usize;
        let mutator_set_hash_encoded = (0..mutator_set_hash_size)
            .map(|i| {
                *memory
                    .get(&(address + BFieldElement::new(1 + i as u64)))
                    .unwrap()
            })
            .collect_vec();
        let mutator_set_hash_hash = VmHasher::hash_varlen(&mutator_set_hash_encoded);
        address += BFieldElement::one() + BFieldElement::new(mutator_set_hash_size as u64);

        // padding
        let zero = Digest::default();

        // Merkleize
        let leafs = [
            inputs_hash,
            outputs_hash,
            pubscript_hashes_and_inputs_hash,
            fee_hash,
            coinbase_hash,
            timestamp_hash,
            mutator_set_hash_hash,
            zero,
        ];
        let tree = <CpuParallel as MerkleTreeMaker<VmHasher>>::from_digests(&leafs);
        let root = tree.get_root();

        // populate memory with merkle tree
        let list_address = rust_shadowing_helper_functions::dyn_malloc::dynamic_allocator(
            16 * DIGEST_LENGTH,
            memory,
        );
        rust_shadowing_helper_functions::unsafe_list::unsafe_list_new(list_address, memory);
        rust_shadowing_helper_functions::unsafe_list::unsafe_list_set_length(
            list_address,
            16,
            memory,
        );
        for (i, node) in tree.nodes.into_iter().enumerate().skip(1) {
            for j in 0..DIGEST_LENGTH {
                memory.insert(
                    list_address
                        + BFieldElement::one()
                        + BFieldElement::new((i * DIGEST_LENGTH + j) as u64),
                    node.values()[j],
                );
            }
        }

        // write digest to stack
        stack.push(root.values()[4]);
        stack.push(root.values()[3]);
        stack.push(root.values()[2]);
        stack.push(root.values()[1]);
        stack.push(root.values()[0]);
    }
}

#[cfg(test)]
mod tests {
    use crate::test_helpers::rust_tasm_equivalence_prop_new;

    use super::TransactionKernelMastHash;

    #[test]
    fn new_prop_test() {
        rust_tasm_equivalence_prop_new(&TransactionKernelMastHash, true);
    }
}

#[cfg(test)]
mod benches {
    use super::*;
    use crate::snippet_bencher::bench_and_write;

    #[test]
    fn get_transaction_kernel_field_benchmark() {
        bench_and_write(TransactionKernelMastHash)
    }
}
