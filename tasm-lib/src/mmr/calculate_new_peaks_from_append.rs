use std::collections::HashMap;

use itertools::Itertools;
use num::One;
use rand::random;
use triton_vm::prelude::*;
use triton_vm::twenty_first::prelude::AlgebraicHasher;
use triton_vm::twenty_first::prelude::Mmr;
use twenty_first::shared_math::other::random_elements;
use twenty_first::util_types::mmr;
use twenty_first::util_types::mmr::mmr_accumulator::MmrAccumulator;

use crate::arithmetic::u64::incr_u64::IncrU64;
use crate::arithmetic::u64::index_of_last_nonzero_bit::IndexOfLastNonZeroBitU64;
use crate::data_type::DataType;
use crate::empty_stack;
use crate::library::Library;
use crate::list::new::New;
use crate::list::pop::Pop;
use crate::list::push::Push;
use crate::list::set_length::SetLength;
use crate::memory::dyn_malloc;
use crate::rust_shadowing_helper_functions;
use crate::traits::deprecated_snippet::DeprecatedSnippet;
use crate::ExecutionState;
use crate::VmHasher;
use crate::DIGEST_LENGTH;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct CalculateNewPeaksFromAppend;

impl CalculateNewPeaksFromAppend {
    fn prepare_state_with_mmra(
        &self,
        start_mmr: MmrAccumulator<VmHasher>,
        new_leaf: Digest,
    ) -> ExecutionState {
        // We assume that the peaks can safely be stored in memory on address 1
        let peaks_pointer = BFieldElement::one();

        let mut stack = empty_stack();
        let old_leaf_count: u64 = start_mmr.count_leaves();
        stack.push(BFieldElement::new(old_leaf_count >> 32));
        stack.push(BFieldElement::new(old_leaf_count & u32::MAX as u64));
        stack.push(peaks_pointer);

        // push digests such that element 0 of digest is on top of stack
        for value in new_leaf.values().iter().rev() {
            stack.push(*value);
        }

        // Initialize memory
        let mut memory: HashMap<BFieldElement, BFieldElement> = HashMap::default();
        rust_shadowing_helper_functions::list::list_new(peaks_pointer, &mut memory);
        for peak in start_mmr.get_peaks() {
            rust_shadowing_helper_functions::list::list_push(
                peaks_pointer,
                peak.values().to_vec(),
                &mut memory,
                DIGEST_LENGTH,
            );
        }

        ExecutionState::with_stack_and_memory(stack, memory)
    }
}

impl DeprecatedSnippet for CalculateNewPeaksFromAppend {
    fn entrypoint_name(&self) -> String {
        "tasm_mmr_calculate_new_peaks_from_append".into()
    }

    fn input_field_names(&self) -> Vec<String> {
        vec![
            "old_leaf_count_hi".to_string(),
            "old_leaf_count_lo".to_string(),
            "*peaks".to_string(),
            "digest_elem_4".to_string(),
            "digest_elem_3".to_string(),
            "digest_elem_2".to_string(),
            "digest_elem_1".to_string(),
            "digest_elem_0".to_string(),
        ]
    }

    fn input_types(&self) -> Vec<DataType> {
        vec![
            DataType::U64,
            DataType::List(Box::new(DataType::Digest)),
            DataType::Digest,
        ]
    }

    fn output_field_names(&self) -> Vec<String> {
        vec!["*new_peaks".to_string(), "*auth_path".to_string()]
    }

    fn output_types(&self) -> Vec<DataType> {
        vec![
            DataType::List(Box::new(DataType::Digest)),
            DataType::List(Box::new(DataType::Digest)),
        ]
    }

    fn stack_diff(&self) -> isize {
        // pops: `old_leaf_count` (u32s<2>); old_peaks (*list); [digests (new_leaf)]
        // pushes: *list (new peaks); *auth_path_of_newly_added_leaf
        -6
    }

    fn function_code(&self, library: &mut Library) -> String {
        let entrypoint = self.entrypoint_name();
        let while_loop_label = format!("{entrypoint}_while");

        let new_list = library.import(Box::new(New::new(DataType::Digest)));
        let push = library.import(Box::new(Push::new(DataType::Digest)));
        let pop = library.import(Box::new(Pop::new(DataType::Digest)));
        let set_length = library.import(Box::new(SetLength::new(DataType::Digest)));
        let u64incr = library.import(Box::new(IncrU64));
        let right_lineage_count = library.import(Box::new(IndexOfLastNonZeroBitU64));

        triton_asm!(
                // BEFORE: _ old_leaf_count_hi old_leaf_count_lo *peaks [digests (new_leaf)]
                // AFTER:  _ *new_peaks *auth_path
                {entrypoint}:
                    dup 5 dup 5 dup 5 dup 5 dup 5 dup 5
                    call {push}
                    pop 5
                    // stack: _ old_leaf_count_hi old_leaf_count_lo *peaks

                    // Create auth_path return value (vector living in RAM)
                    call {new_list}
                    push 0
                    call {set_length}
                    // stack: _ old_leaf_count_hi old_leaf_count_lo *peaks *auth_path

                    swap 1
                    // stack: _ old_leaf_count_hi old_leaf_count_lo *auth_path *peaks

                    dup 3 dup 3
                    // stack: _ old_leaf_count_hi old_leaf_count_lo *auth_path *peaks old_leaf_count_hi old_leaf_count_lo

                    call {u64incr}
                    call {right_lineage_count}

                    call {while_loop_label}
                    // stack: _ old_leaf_count_hi old_leaf_count_lo *auth_path *peaks 0

                    pop 1
                    swap 3 pop 1 swap 1 pop 1
                    // stack: _ *peaks *auth_path

                    return

                // INVARIANT: _ old_leaf_count_hi old_leaf_count_lo *auth_path *peaks rll
                {while_loop_label}:
                    dup 0
                    push 0
                    eq
                    skiz
                        return
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo *auth_path *peaks rll

                    swap 2 swap 1
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks

                    dup 0
                    dup 0
                    call {pop}
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks *peaks [digest (new_hash)]

                    dup 5
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks *peaks [digest (new_hash)] *peaks

                    call {pop}
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks *peaks [digest (new_hash)] [digests (previous_peak)]

                    // Update authentication path with latest previous_peak
                    dup 12
                    dup 5 dup 5 dup 5 dup 5 dup 5
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks *peaks [digest (new_hash)] [digests (previous_peak)] *auth_path [digests (previous_peak)]

                    call {push}
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks *peaks [digest (new_hash)] [digests (previous_peak)]

                    hash
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks *peaks [digests (new_peak)]

                    call {push}
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo rll *auth_path *peaks

                    swap 1 swap 2
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo *auth_path *peaks rll

                    push -1
                    add
                    // Stack: _ old_leaf_count_hi old_leaf_count_lo *auth_path *peaks (rll - 1)

                    recurse
        )
        .iter()
        .join("\n")
    }

    fn crash_conditions(&self) -> Vec<String> {
        vec!["Snippet arguments are not a valid MMR accumulator".to_string()]
    }

    fn gen_input_states(&self) -> Vec<ExecutionState> {
        let mut ret = vec![];
        for mmr_size in [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 100, 1000,
        ] {
            let digests: Vec<Digest> = random_elements(mmr_size);
            let new_leaf: Digest = random();
            let mmra = MmrAccumulator::new(digests);
            ret.push(self.prepare_state_with_mmra(mmra, new_leaf));
        }

        ret
    }

    fn common_case_input_state(&self) -> ExecutionState {
        let peaks: Vec<Digest> = random_elements(31);
        let new_leaf: Digest = random();
        let mmra = MmrAccumulator::init(peaks, (1 << 31) - 1);
        self.prepare_state_with_mmra(mmra, new_leaf)
    }

    fn worst_case_input_state(&self) -> ExecutionState {
        let peaks: Vec<Digest> = random_elements(62);
        let new_leaf: Digest = random();
        let mmra = MmrAccumulator::init(peaks, (1 << 62) - 1);
        self.prepare_state_with_mmra(mmra, new_leaf)
    }

    fn rust_shadowing(
        &self,
        stack: &mut Vec<BFieldElement>,
        _std_in: Vec<BFieldElement>,
        _secret_in: Vec<BFieldElement>,
        memory: &mut HashMap<BFieldElement, BFieldElement>,
    ) {
        // BEFORE: _ old_leaf_count_hi old_leaf_count_lo *peaks [digests (new_leaf)]
        // AFTER:  _ *new_peaks *auth_path
        let new_leaf: Digest = Digest::new([
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
            stack.pop().unwrap(),
        ]);
        let peaks_pointer = stack.pop().unwrap();
        let old_leaf_count_lo = stack.pop().unwrap().value();
        let old_leaf_count_hi = stack.pop().unwrap().value();
        let old_leaf_count = (old_leaf_count_hi << 32) | old_leaf_count_lo;

        let mut old_peaks: Vec<Digest> = vec![];
        let peak_count = memory[&peaks_pointer].value() as u32;

        let list_push = rust_shadowing_helper_functions::list::list_push;
        let list_pop = rust_shadowing_helper_functions::list::list_pop;

        for i in 0..peak_count {
            old_peaks.push(Digest::new(
                rust_shadowing_helper_functions::list::list_get(
                    peaks_pointer,
                    i as usize,
                    memory,
                    DIGEST_LENGTH,
                )
                .try_into()
                .unwrap(),
            ));
        }

        let auth_path_pointer = dyn_malloc::DYN_MALLOC_FIRST_ADDRESS;
        rust_shadowing_helper_functions::list::list_new(auth_path_pointer, memory);
        list_push(
            peaks_pointer,
            new_leaf.values().to_vec(),
            memory,
            DIGEST_LENGTH,
        );
        let new_node_index = mmr::shared_advanced::leaf_index_to_node_index(old_leaf_count);
        let (mut right_lineage_count, _height) =
            mmr::shared_advanced::right_lineage_length_and_own_height(new_node_index);
        while right_lineage_count != 0 {
            let new_hash = Digest::new(
                list_pop(peaks_pointer, memory, DIGEST_LENGTH)
                    .try_into()
                    .unwrap(),
            );
            let previous_peak = Digest::new(
                list_pop(peaks_pointer, memory, DIGEST_LENGTH)
                    .try_into()
                    .unwrap(),
            );
            list_push(
                auth_path_pointer,
                previous_peak.values().to_vec(),
                memory,
                DIGEST_LENGTH,
            );
            list_push(
                peaks_pointer,
                VmHasher::hash_pair(previous_peak, new_hash)
                    .values()
                    .to_vec(),
                memory,
                DIGEST_LENGTH,
            );
            right_lineage_count -= 1;
        }

        // Pop return values to stack
        stack.push(peaks_pointer);
        stack.push(auth_path_pointer); // Can this be done in a more dynamic way?
    }
}

#[cfg(test)]
mod tests {
    use num::Zero;
    use triton_vm::twenty_first::prelude::MmrMembershipProof;
    use twenty_first::shared_math::other::random_elements;
    use twenty_first::util_types::mmr::mmr_accumulator::MmrAccumulator;

    use crate::test_helpers::test_rust_equivalence_given_input_values_deprecated;
    use crate::test_helpers::test_rust_equivalence_multiple_deprecated;

    use super::*;

    type Mmra = MmrAccumulator<VmHasher>;

    #[test]
    fn calculate_new_peaks_from_append_test_lists() {
        test_rust_equivalence_multiple_deprecated(&CalculateNewPeaksFromAppend, true);
    }

    #[test]
    fn mmr_sanity_check_new_and_init() {
        for mmr_size in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 14, 100, 1000] {
            let digests: Vec<Digest> = random_elements(mmr_size);
            let mmr_by_new: Mmra = MmrAccumulator::new(digests);
            let mmr_by_init: Mmra = MmrAccumulator::init(mmr_by_new.get_peaks(), mmr_size as u64);
            assert_eq!(mmr_by_new, mmr_by_init);
        }
    }

    #[test]
    fn mmra_append_test_empty() {
        let mmra: Mmra = MmrAccumulator::new(vec![]);
        let digest = VmHasher::hash(&BFieldElement::zero());
        let expected_final_mmra = MmrAccumulator::new(vec![digest]);
        prop_calculate_new_peaks_from_append(mmra, digest, expected_final_mmra);
    }

    #[test]
    fn mmra_append_test_single() {
        let digest0 = VmHasher::hash(&BFieldElement::new(4545));
        let digest1 = VmHasher::hash(&BFieldElement::new(12345));
        let mmra: Mmra = MmrAccumulator::new(vec![digest0]);
        let expected_final_mmra = MmrAccumulator::new(vec![digest0, digest1]);
        prop_calculate_new_peaks_from_append(mmra, digest1, expected_final_mmra);
    }

    #[test]
    fn mmra_append_test_two_leaves() {
        let digest0 = VmHasher::hash(&BFieldElement::new(4545));
        let digest1 = VmHasher::hash(&BFieldElement::new(12345));
        let digest2 = VmHasher::hash(&BFieldElement::new(55488));
        let mmra: Mmra = MmrAccumulator::new(vec![digest0, digest1]);
        let expected_final_mmra = MmrAccumulator::new(vec![digest0, digest1, digest2]);
        prop_calculate_new_peaks_from_append(mmra, digest2, expected_final_mmra);
    }
    #[test]
    fn mmra_append_test_three_leaves() {
        let digest0 = VmHasher::hash(&BFieldElement::new(4545));
        let digest1 = VmHasher::hash(&BFieldElement::new(12345));
        let digest2 = VmHasher::hash(&BFieldElement::new(55488));
        let digest3 = VmHasher::hash(&BFieldElement::new(554880000000));
        let mmra: Mmra = MmrAccumulator::new(vec![digest0, digest1, digest2]);
        let expected_final_mmra = MmrAccumulator::new(vec![digest0, digest1, digest2, digest3]);
        prop_calculate_new_peaks_from_append(mmra, digest3, expected_final_mmra);
    }

    #[test]
    fn mmra_append_pbt() {
        let inserted_digest: Digest = VmHasher::hash(&BFieldElement::new(1337));
        for init_size in 0..40 {
            println!("init_size = {init_size}");
            let leaf_digests: Vec<Digest> = random_elements(init_size);
            let init_mmra: Mmra = MmrAccumulator::new(leaf_digests.clone());
            let expected_final_mmra: Mmra =
                MmrAccumulator::new([leaf_digests, vec![inserted_digest]].concat());
            prop_calculate_new_peaks_from_append(init_mmra, inserted_digest, expected_final_mmra);
        }
    }

    #[test]
    fn mmra_append_big_mmr() {
        // Set MMR to be with 2^32 - 1 leaves and 32 peaks. Prepending one leaf should then reduce the number of leaves to 1.
        let inserted_digest: Digest = VmHasher::hash(&BFieldElement::new(1337));
        let init_mmra: Mmra = MmrAccumulator::init(
            vec![VmHasher::hash(&BFieldElement::zero()); 32],
            (1 << 32) - 1,
        );
        let mut expected_final_mmr = init_mmra.clone();
        expected_final_mmr.append(inserted_digest);
        prop_calculate_new_peaks_from_append(init_mmra, inserted_digest, expected_final_mmr);

        // Set MMR to be with 2^33 - 1 leaves and 33 peaks. Prepending one leaf should then reduce the number of leaves to 1.
        let inserted_digest: Digest = VmHasher::hash(&BFieldElement::new(1337));
        let init_mmra: Mmra = MmrAccumulator::init(
            vec![VmHasher::hash(&BFieldElement::zero()); 33],
            (1 << 33) - 1,
        );
        let mut expected_final_mmr = init_mmra.clone();
        expected_final_mmr.append(inserted_digest);
        prop_calculate_new_peaks_from_append(init_mmra, inserted_digest, expected_final_mmr);
    }

    fn prop_calculate_new_peaks_from_append(
        start_mmr: MmrAccumulator<VmHasher>,
        new_leaf: Digest,
        expected_mmr: MmrAccumulator<VmHasher>,
    ) {
        // We assume that the peaks can safely be stored in memory on address 0
        let peaks_pointer = BFieldElement::one();

        // BEFORE: _ old_leaf_count_hi old_leaf_count_lo *peaks [digests (new_leaf)]
        // AFTER:  _ *new_peaks *auth_path
        let mut init_stack = empty_stack();
        let old_leaf_count: u64 = start_mmr.count_leaves();
        init_stack.push(BFieldElement::new(old_leaf_count >> 32));
        init_stack.push(BFieldElement::new(old_leaf_count & u32::MAX as u64));
        init_stack.push(peaks_pointer);

        // push digests such that element 0 of digest is on top of stack
        for value in new_leaf.values().iter().rev() {
            init_stack.push(*value);
        }

        // Initialize memory
        let mut memory: HashMap<BFieldElement, BFieldElement> = HashMap::default();
        rust_shadowing_helper_functions::list::list_new(peaks_pointer, &mut memory);

        let list_get = rust_shadowing_helper_functions::list::list_get;
        for peak in start_mmr.get_peaks() {
            rust_shadowing_helper_functions::list::list_push(
                peaks_pointer,
                peak.values().to_vec(),
                &mut memory,
                DIGEST_LENGTH,
            );
        }

        let auth_paths_pointer = dyn_malloc::DYN_MALLOC_FIRST_ADDRESS;
        let mut expected_final_stack = empty_stack();
        expected_final_stack.push(peaks_pointer);
        expected_final_stack.push(auth_paths_pointer);

        let vm_output = test_rust_equivalence_given_input_values_deprecated(
            &CalculateNewPeaksFromAppend,
            &init_stack,
            &[],
            memory,
            Some(&expected_final_stack),
        );

        // Find produced MMR
        let final_memory = vm_output.ram;
        let peaks_count = final_memory[&peaks_pointer].value();
        let mut produced_peaks = vec![];
        for i in 0..peaks_count {
            let peak = Digest::new(
                list_get(peaks_pointer, i as usize, &final_memory, DIGEST_LENGTH)
                    .try_into()
                    .unwrap(),
            );
            produced_peaks.push(peak);
        }

        let produced_mmr: Mmra = MmrAccumulator::init(produced_peaks, start_mmr.count_leaves() + 1);

        // Verify that both code paths produce the same MMR
        assert_eq!(expected_mmr, produced_mmr);

        // Verify that produced auth paths are valid
        let auth_path_element_count = final_memory[&auth_paths_pointer].value();
        let mut produced_auth_path = vec![];
        for i in 0..auth_path_element_count {
            produced_auth_path.push(Digest::new(
                list_get(auth_paths_pointer, i as usize, &final_memory, DIGEST_LENGTH)
                    .try_into()
                    .unwrap(),
            ));
        }

        let produced_mp = MmrMembershipProof::<VmHasher> {
            leaf_index: start_mmr.count_leaves(),
            authentication_path: produced_auth_path,
            _hasher: std::marker::PhantomData,
        };
        assert!(
            produced_mp
                .verify(
                    &produced_mmr.get_peaks(),
                    new_leaf,
                    produced_mmr.count_leaves(),
                )
                .0,
            "TASM-produced authentication path must be valid"
        );
    }
}

#[cfg(test)]
mod benches {
    use crate::snippet_bencher::bench_and_write;

    use super::*;

    #[test]
    fn calculate_new_peaks_from_append_benchmark() {
        bench_and_write(CalculateNewPeaksFromAppend);
    }
}
