use triton_vm::{
    program::NonDeterminism,
    triton_asm,
    twenty_first::util_types::mmr::{
        mmr_accumulator::MmrAccumulator, mmr_successor_proof::MmrSuccessorProof,
    },
};

use crate::{
    arithmetic::u64::{
        add_u64::AddU64, log_2_floor_u64::Log2FloorU64, lt_u64::LtU64ConsumeArgs,
        popcount_u64::PopCountU64, sub_u64::SubU64,
    },
    data_type::DataType,
    field,
    hashing::merkle_step_u64_index::MerkleStepU64Index,
    mmr::leaf_index_to_mt_index_and_peak_index::MmrLeafIndexToMtIndexAndPeakIndex,
    prelude::BasicSnippet,
    Digest,
};

/// Verify a the scucessorship relation between two MMRs. A `MmrSuccessorProof`
/// is necessary to demonstrate this relation, but it is not a *stack* argument
/// because this algorithm obtains the relevant info (authentication paths) from
/// nondeterministic digests. Accordingly, nondeterminism must be initialized
/// correctly with the `MmrSuccessorProof`.
///
/// This snippet crashes if the relation does not hold (or if the proof is invalid).
pub struct VerifyMmrSuccessor;

impl BasicSnippet for VerifyMmrSuccessor {
    fn inputs(&self) -> Vec<(crate::data_type::DataType, String)> {
        vec![
            (DataType::VoidPointer, "*old_mmr".to_string()),
            (DataType::VoidPointer, "*new_mmr".to_string()),
        ]
    }

    fn outputs(&self) -> Vec<(crate::data_type::DataType, String)> {
        vec![]
    }

    fn entrypoint(&self) -> String {
        "tasm_lib_mmr_verify_mmr_successor".to_string()
    }

    fn code(
        &self,
        library: &mut crate::prelude::Library,
    ) -> Vec<triton_vm::prelude::LabelledInstruction> {
        let field_peaks = field!(MmrAccumulator::peaks);
        let field_leaf_count = field!(MmrAccumulator::leaf_count);
        let num_peaks = triton_asm! {
            // _ *mmr
            {&field_peaks}
            // _ *peaks
            read_mem 1
            // _ len (*peaks-1)
            pop 1
            // _ len
        };
        let num_leafs = triton_asm!(
            // _ *mmr
            {&field_leaf_count}
            // _ *leaf_count
            push 1 add read_mem 2
            // _ [num_leafs] (*leaf_count-1)
            pop 1
        );
        let ltu64 = library.import(Box::new(LtU64ConsumeArgs));
        let popcount_u64 = library.import(Box::new(PopCountU64));
        let addu64 = library.import(Box::new(AddU64));
        let leaf_index_to_mti_and_pki = library.import(Box::new(MmrLeafIndexToMtIndexAndPeakIndex));
        let merkle_step_u64 = library.import(Box::new(MerkleStepU64Index));
        let ilog2_u64 = library.import(Box::new(Log2FloorU64));
        let sub_u64 = library.import(Box::new(SubU64));
        let compare_digests = DataType::Digest.compare();
        let compare_u64 = DataType::U64.compare();
        let entrypoint = self.entrypoint();
        let main_loop = format!("{entrypoint}_main_loop");
        let traverse = format!("{entrypoint}_traverse_partial_auth_path");

        let strip_top_bit = triton_asm!(
            // BEFORE: [num_leafs_remaining] garbage
            // AFTER: [num_leafs_remaining*] old_height
            pop 1
            dup 1 dup 1 call {ilog2_u64}
            // [num_leafs_remaining] old_height
            dup 2 push 2 pow split
            // [num_leafs_remaining] old_height [1<<old_height]
            dup 4 dup 4
            // [num_leafs_remaining] old_height [1<<old_height] [num_leafs_remaining]
            call {sub_u64}
            // [num_leafs_remaining] old_height [num_leafs_remaining*]
            swap 3 swap 1 swap 4
            // [num_leafs_remaining*] old_height [num_leafs_remaining]
            pop 2
            // [num_leafs_remaining*] old_height
        );

        triton_asm! {
            // BEFORE: _ *old_mmr *new_mmr
            // AFTER: _
            {entrypoint}:

            /* tests before preparing loop */

            // new num leafs < old num leafs  ?
            dup 1 {&num_leafs}
            // _ *old_mmr *new_mmr [old_num_leafs]

            dup 2 {&num_leafs}
            // _ *old_mmr *new_mmr [old_num_leafs] [new_num_leafs]

            call {ltu64}
            // _ *old_mmr *new_mmr (new_num_leafs < old_num_leafs)

            push 0 eq
            // _ *old_mmr *new_mmr (new_num_leafs >= old_num_leafs)

            assert
            // _ *old_mmr *new_mmr

            // consistent new mmr?
            dup 0 {&num_peaks}
            // _ *old_mmr *new_mmr new_num_peaks

            dup 1 {&num_leafs} call {popcount_u64}
            // _ *old_mmr *new_mmr new_num_peaks (popcount of new_num_leafs)

            eq assert
            // _ *old_mmr *new_mmr


            /* prepare and call loop */
            dup 1 {&field_peaks}
            // _ *old_mmr *new_mmr *old_peaks

            read_mem 1 push 2 add
            // _ *old_mmr *new_mmr num_old_peaks *old_peaks[0]

            swap 1 push {Digest::LEN} mul
            // _ *old_mmr *new_mmr *old_peaks[0] (num_old_peaks*5)

            dup 1 add
            // _ *old_mmr *new_mmr *old_peaks[0] *end_of_memory

            push 0 push 0
            // _ *old_mmr *new_mmr *old_peaks[0] *end_of_memory [0]

            dup 6 {&num_leafs}
            // _ *old_mmr *new_mmr *old_peaks[0] *end_of_memory [0] [old_num_leafs]

            push {0x455b00b5}
            // _ *old_mmr *new_mmr *old_peaks[0] *end_of_memory [0] [old_num_leafs] garbage

            call {main_loop}
            // _ *old_mmr *new_mmr *end_of_memory *end_of_memory [old_num_leafs] [0] garbage

            /* clean up after loop */
            pop 5
            // _ *old_mmr *new_mmr *end_of_memory *end_of_memory

            pop 4
            // _

            return

            // INVARIANT: _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining] garbage
            {main_loop}:
                {&strip_top_bit}
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height

                dup 7 {&num_leafs}
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height [new_num_leafs]

                dup 6 dup 6
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height [new_num_leafs] [running_leaf_count]

                call {leaf_index_to_mti_and_pki}
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height [merkle_tree_index] peak_index

                swap 2 swap 1
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height peak_index [merkle_tree_index]


                /* prepare & traverse */
                dup 9 push {Digest::LEN-1} read_mem {Digest::LEN} pop 1
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height peak_index [merkle_tree_index] [current_old_peak]

                call {traverse}
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height peak_index [1] [some_new_peak]

                dup 15 {&field_peaks}
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height peak_index [1] [some_new_peak] *new_peaks

                push {1 + Digest::LEN - 1} dup 7 push {Digest::LEN} mul add
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height peak_index [1] [some_new_peak] *new_peaks (5+peak_index*5)

                add read_mem {Digest::LEN} pop 1
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height peak_index [1] [some_new_peak] [new_peaks[peak_index]]

                {&compare_digests} assert
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height peak_index [1]


                /* prepare for next iteration */
                pop 3
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] old_height

                push 2 pow split
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] [1<<old_height]

                dup 5 dup 5 call {addu64}
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count] [num_leafs_remaining*] [running_leaf_count*]

                swap 4 swap 1 swap 5 pop
                // _ *new_mmr *current_peak *end_of_memory [running_leaf_count*] [num_leafs_remaining*] garbage

                swap 6 push {Digest::LEN} add swap 6
                // _ *new_mmr *next_peak *end_of_memory [running_leaf_count*] [num_leafs_remaining*] garbage

                return_or_recurse

            // INVARIANT: _ [merkle_tree_index] [current_node]
            {traverse}:
                // evaluate termination condition
                dup 6 dup 6 push 0 push 1
                // _ [merkle_tree_index] [current_node] [merkle_tree_index] [1]

                {&compare_u64}
                // _ [merkle_tree_index] [current_node] (merkle_tree_index == 1)

                skiz return
                // _ [merkle_tree_index] [current_node]

                call {merkle_step_u64}
                // _ [merkle_tree_index] [current_node*]

                recurse
        }
    }
}

impl VerifyMmrSuccessor {
    /// Update a nondeterminism in accordance with verifying a given `MmrSuccessorProof`
    /// with this snippet.
    pub fn update_nondeterminism(
        nondeterminism: &mut NonDeterminism,
        mmr_successor_proof: &MmrSuccessorProof,
    ) {
        nondeterminism
            .digests
            .append(&mut mmr_successor_proof.paths.clone())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use itertools::Itertools;
    use rand::prelude::StdRng;
    use rand::Rng;
    use rand::RngCore;
    use rand::SeedableRng;
    use triton_vm::{
        prelude::BFieldElement,
        program::NonDeterminism,
        twenty_first::{
            prelude::Mmr,
            util_types::mmr::{
                mmr_accumulator::MmrAccumulator, mmr_successor_proof::MmrSuccessorProof,
                shared_advanced::get_peak_heights,
                shared_basic::leaf_index_to_mt_index_and_peak_index,
            },
        },
    };

    use crate::empty_stack;
    use crate::memory::encode_to_memory;
    use crate::memory::FIRST_NON_DETERMINISTICALLY_INITIALIZED_MEMORY_ADDRESS;
    use crate::traits::algorithm::ShadowedAlgorithm;
    use crate::traits::rust_shadow::RustShadow;
    use crate::{
        prelude::TasmObject,
        snippet_bencher::BenchmarkCase,
        traits::algorithm::{Algorithm, AlgorithmInitialState},
        Digest,
    };

    use super::VerifyMmrSuccessor;

    fn num_digests_to_read(old_mmr: &MmrAccumulator, new_mmr: &MmrAccumulator) -> usize {
        let mut number = 0;
        let mut running_leaf_count = 0;
        let old_peak_heights = get_peak_heights(old_mmr.num_leafs());
        let mut new_merkle_tree_indices_of_old_peaks = vec![];
        for old_peak_height in old_peak_heights {
            let (merkle_tree_index, _peak_index) =
                leaf_index_to_mt_index_and_peak_index(running_leaf_count, new_mmr.num_leafs());
            running_leaf_count += 1 << old_peak_height;
            new_merkle_tree_indices_of_old_peaks.push(merkle_tree_index);
        }
        for mut merkle_tree_index in new_merkle_tree_indices_of_old_peaks {
            while merkle_tree_index != 1 {
                number += 1;
                merkle_tree_index >>= 1;
            }
        }
        number
    }

    impl Algorithm for VerifyMmrSuccessor {
        fn rust_shadow(
            &self,
            stack: &mut Vec<BFieldElement>,
            memory: &mut HashMap<BFieldElement, BFieldElement>,
            nondeterminism: &NonDeterminism,
        ) {
            let new_mmr_pointer = stack.pop().unwrap();
            let old_mmr_pointer = stack.pop().unwrap();

            let new_mmr = *MmrAccumulator::decode_from_memory(memory, new_mmr_pointer).unwrap();
            let old_mmr = *MmrAccumulator::decode_from_memory(memory, old_mmr_pointer).unwrap();

            let num_digests = num_digests_to_read(&old_mmr, &new_mmr);

            let digests = nondeterminism.digests[0..num_digests].to_vec();
            let mmr_successor_proof = MmrSuccessorProof { paths: digests };

            assert!(mmr_successor_proof.verify(&old_mmr, &new_mmr));
        }

        fn pseudorandom_initial_state(
            &self,
            seed: [u8; 32],
            bench_case: Option<BenchmarkCase>,
        ) -> AlgorithmInitialState {
            let mut rng: StdRng = SeedableRng::from_seed(seed);
            let old_num_leafs = rng.next_u64() & (u64::MAX >> 1);
            let old_peaks = (0..old_num_leafs.count_ones())
                .map(|_| rng.gen::<Digest>())
                .collect_vec();
            let old_mmr = MmrAccumulator::init(old_peaks, old_num_leafs);

            let num_new_leafs = match bench_case {
                Some(BenchmarkCase::CommonCase) => rng.gen_range(0..10),
                Some(BenchmarkCase::WorstCase) => rng.gen_range(0..15),
                None => rng.gen_range(0..5),
            };
            let new_leafs = (0..num_new_leafs)
                .map(|_| rng.gen::<Digest>())
                .collect_vec();
            let mmr_successor_proof =
                MmrSuccessorProof::new_from_batch_append(&old_mmr, &new_leafs);
            println!(
                "produced mmr successor proof of {} digests",
                mmr_successor_proof.paths.len()
            );
            let mut new_mmr = old_mmr.clone();
            for leaf in new_leafs {
                new_mmr.append(leaf);
            }

            let mut nondeterminism = NonDeterminism::new(vec![]);
            VerifyMmrSuccessor::update_nondeterminism(&mut nondeterminism, &mmr_successor_proof);
            let old_mmr_address = FIRST_NON_DETERMINISTICALLY_INITIALIZED_MEMORY_ADDRESS;
            let new_mmr_address =
                encode_to_memory(&mut nondeterminism.ram, old_mmr_address, old_mmr);
            let _garbage_address =
                encode_to_memory(&mut nondeterminism.ram, new_mmr_address, new_mmr);
            let mut stack = empty_stack();
            stack.push(old_mmr_address);
            stack.push(new_mmr_address);
            AlgorithmInitialState {
                stack,
                nondeterminism,
            }
        }
    }

    #[test]
    fn verify_mmr_successor_simple_test() {
        ShadowedAlgorithm::new(VerifyMmrSuccessor).test();
    }
}
