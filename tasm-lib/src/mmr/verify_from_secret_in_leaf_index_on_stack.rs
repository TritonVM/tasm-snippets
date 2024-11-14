use triton_vm::prelude::*;

use super::leaf_index_to_mt_index_and_peak_index::MmrLeafIndexToMtIndexAndPeakIndex;
use crate::arithmetic::u64::eq_u64::EqU64;
use crate::data_type::DataType;
use crate::hashing::merkle_step_u64_index::MerkleStepU64Index;
use crate::library::Library;
use crate::list::get::Get;
use crate::traits::basic_snippet::BasicSnippet;

/// Verify that a digest is a leaf in the MMR accumulator. Takes both authentication path from
/// secret-in. Crashes the VM if the authentication fails.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub struct MmrVerifyFromSecretInLeafIndexOnStack;

impl BasicSnippet for MmrVerifyFromSecretInLeafIndexOnStack {
    fn inputs(&self) -> Vec<(DataType, String)> {
        vec![(
            DataType::Tuple(vec![
                DataType::List(Box::new(DataType::Digest)), // *peaks
                DataType::Digest,                           // leaf
                DataType::U64,                              // leaf_count
                DataType::U64,                              // leaf_index
            ]),
            "peaks_leaf_count_leaf_index_and_leaf".to_owned(),
        )]
    }

    fn outputs(&self) -> Vec<(DataType, String)> {
        vec![]
    }

    fn entrypoint(&self) -> String {
        "tasmlib_mmr_verify_from_secret_in_leaf_index_on_stack".into()
    }

    fn code(&self, library: &mut Library) -> Vec<LabelledInstruction> {
        let entrypoint = self.entrypoint();
        let auth_path_loop_label = format!("{entrypoint}_auth_path_loop");

        let leaf_index_to_mt_index = library.import(Box::new(MmrLeafIndexToMtIndexAndPeakIndex));
        let eq_u64 = library.import(Box::new(EqU64));
        let merkle_step_u64_index = library.import(Box::new(MerkleStepU64Index));
        let list_get = library.import(Box::new(Get::new(DataType::Digest)));

        let auth_path_loop_code = triton_asm!(
            {auth_path_loop_label}:
                dup 6 dup 6 push 0 push 1 call {eq_u64}
                // __ mt_index_hi mt_index_lo [acc_hash] (mt_index == 1)

                skiz return
                // __ mt_index_hi mt_index_lo [acc_hash]

                // move up one layer in the Merkle tree
                call {merkle_step_u64_index}

                // _ (mt_index / 2)_hi (mt_index / 2)_lo [digest (acc_hash)]

                recurse
        );

        triton_asm!(
            {entrypoint}:
                // _ *peaks [leaf] [leaf_count] [leaf_index]

                call {leaf_index_to_mt_index}
                // _ *peaks [leaf] mt_index_hi mt_index_lo peak_index

                place 7 place 6 place 6
                // _ *peaks peak_index mt_index_hi mt_index_lo [digest (leaf_digest)]

                call {auth_path_loop_label}
                // _ *peaks peak_index 0 1 [acc_hash_result]

                dup 8 dup 8 call {list_get}
                // _ *peaks peak_index 0 1 [acc_hash_result] [expected_root]

                assert_vector error_id 10
                // _ *peaks peak_index 0 1 [acc_hash_result]

                pop 5
                pop 4
                // _


                return

            {&auth_path_loop_code}
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest_arbitrary_interop::arb;
    use rand::prelude::*;
    use tasm_lib::test_helpers::test_assertion_failure;
    use test_strategy::proptest;
    use twenty_first::math::other::random_elements;
    use twenty_first::util_types::mmr::mmr_accumulator::util::mmra_with_mps;
    use twenty_first::util_types::mmr::mmr_accumulator::MmrAccumulator;
    use twenty_first::util_types::mmr::mmr_membership_proof::MmrMembershipProof;
    use twenty_first::util_types::mmr::mmr_trait::Mmr;
    use twenty_first::util_types::mmr::shared_basic::leaf_index_to_mt_index_and_peak_index;

    use super::*;
    use crate::rust_shadowing_helper_functions;
    use crate::snippet_bencher::BenchmarkCase;
    use crate::traits::procedure::Procedure;
    use crate::traits::procedure::ProcedureInitialState;
    use crate::traits::procedure::ShadowedProcedure;
    use crate::traits::rust_shadow::RustShadow;
    use crate::VmHasher;

    #[test]
    fn prop() {
        ShadowedProcedure::new(MmrVerifyFromSecretInLeafIndexOnStack).test();
    }

    #[proptest(cases = 32)]
    fn negative_test_bad_leaf_index(
        #[strategy(0_u64..1 << 62)] leaf_count: u64,
        #[strategy(0_u64..#leaf_count)] real_leaf_index: u64,
        #[strategy(0..#leaf_count)]
        #[filter(#real_leaf_index != #bad_leaf_index)]
        bad_leaf_index: u64,
        #[strategy(arb())] leaf: Digest,
        #[strategy(arb())] peaks_pointer: BFieldElement,
    ) {
        let (mmr, mps) = mmra_with_mps(leaf_count, vec![(real_leaf_index, leaf)]);
        let auth_path = mps[0].authentication_path.clone();

        // Extend the auth path to ensure that execution does not run out of digests in non-
        // determinism since this would result in another error code in TVM than the one we intend
        // to get: vector assertion error.
        let padded_auth_path = [auth_path.clone(), random_elements(64)].concat();
        let init_state = MmrVerifyFromSecretInLeafIndexOnStack.prepare_state(
            &mmr,
            peaks_pointer,
            bad_leaf_index,
            leaf,
            padded_auth_path,
        );

        test_assertion_failure(
            &ShadowedProcedure::new(MmrVerifyFromSecretInLeafIndexOnStack),
            init_state.into(),
            &[10],
        );

        // Sanity check
        assert!(!MmrMembershipProof::new(auth_path).verify(
            bad_leaf_index,
            leaf,
            &mmr.peaks(),
            mmr.num_leafs()
        ));
    }

    impl Procedure for MmrVerifyFromSecretInLeafIndexOnStack {
        fn rust_shadow(
            &self,
            stack: &mut Vec<BFieldElement>,
            memory: &mut HashMap<BFieldElement, BFieldElement>,
            nondeterminism: &NonDeterminism,
            _public_input: &[BFieldElement],
            _sponge: &mut Option<VmHasher>,
        ) -> Vec<BFieldElement> {
            let leaf_index_lo: u32 = stack.pop().unwrap().try_into().unwrap();
            let leaf_index_hi: u32 = stack.pop().unwrap().try_into().unwrap();
            let leaf_index = ((leaf_index_hi as u64) << 32) + leaf_index_lo as u64;

            let leaf_count_lo: u32 = stack.pop().unwrap().try_into().unwrap();
            let leaf_count_hi: u32 = stack.pop().unwrap().try_into().unwrap();
            let leaf_count: u64 = ((leaf_count_hi as u64) << 32) + leaf_count_lo as u64;

            let mut leaf_digest = Digest::default();
            for elem in leaf_digest.0.iter_mut() {
                *elem = stack.pop().unwrap();
            }

            let peaks_pointer = stack.pop().unwrap();
            let peaks_count: u64 = memory[&peaks_pointer].value();
            let mut peaks: Vec<Digest> = vec![];
            for i in 0..peaks_count {
                let digest = Digest::new(
                    rust_shadowing_helper_functions::list::list_get(
                        peaks_pointer,
                        i as usize,
                        memory,
                        Digest::LEN,
                    )
                    .try_into()
                    .unwrap(),
                );
                peaks.push(digest);
            }

            let (mut mt_index, _peak_index) =
                leaf_index_to_mt_index_and_peak_index(leaf_index, leaf_count);

            let mut auth_path: Vec<Digest> = vec![];
            let mut i = 0;
            while mt_index != 1 {
                auth_path.push(nondeterminism.digests[i]);
                mt_index /= 2;
                i += 1;
            }

            let valid_mp = MmrMembershipProof::new(auth_path).verify(
                leaf_index,
                leaf_digest,
                &peaks,
                leaf_count,
            );

            assert!(valid_mp, "MMR leaf must authenticate against peak");

            vec![]
        }

        fn pseudorandom_initial_state(
            &self,
            seed: [u8; 32],
            bench_case: Option<BenchmarkCase>,
        ) -> ProcedureInitialState {
            let mut rng: StdRng = SeedableRng::from_seed(seed);

            let (leaf_count, leaf_index) = match bench_case {
                Some(BenchmarkCase::CommonCase) => (1u64 << 32, 1 << 31),
                Some(BenchmarkCase::WorstCase) => (1u64 << 62, 1 << 61),
                None => {
                    let leaf_count = rng.gen_range(0..(1 << 62));
                    let leaf_index = rng.gen_range(0..leaf_count);

                    (leaf_count, leaf_index)
                }
            };

            let peaks_pointer: BFieldElement = rng.gen();
            let valid_leaf: Digest = random();
            let (mmr, mps) = mmra_with_mps(leaf_count, vec![(leaf_index, valid_leaf)]);
            self.prepare_state(
                &mmr,
                peaks_pointer,
                leaf_index,
                valid_leaf,
                mps[0].authentication_path.clone(),
            )
        }
    }

    impl MmrVerifyFromSecretInLeafIndexOnStack {
        fn prepare_state(
            &self,
            mmr: &MmrAccumulator,
            peaks_pointer: BFieldElement,
            claimed_leaf_index: u64,
            claimed_leaf: Digest,
            auth_path: Vec<Digest>,
        ) -> ProcedureInitialState {
            let mut init_state = self.mmr_to_init_vm_state(mmr, peaks_pointer, claimed_leaf);
            init_state.nondeterminism.digests = auth_path;
            let leaf_index_encoded = [
                bfe!(claimed_leaf_index >> 32),
                bfe!(claimed_leaf_index & u32::MAX as u64),
            ];
            init_state.stack.extend(leaf_index_encoded);

            init_state
        }

        /// Prepare the state with the known MMR and the known `claimed_leaf`, caller needs to set
        /// leaf index and auth path.
        fn mmr_to_init_vm_state(
            &self,
            mmra: &MmrAccumulator,
            peaks_pointer: BFieldElement,
            claimed_leaf: Digest,
        ) -> ProcedureInitialState {
            let mut stack: Vec<BFieldElement> = self.init_stack_for_isolated_run();
            stack.push(peaks_pointer);

            for word in claimed_leaf.0.into_iter().rev() {
                stack.push(word);
            }

            let leaf_count = mmra.num_leafs();
            let leaf_count_hi = BFieldElement::new(leaf_count >> 32);
            let leaf_count_lo = BFieldElement::new(leaf_count & u32::MAX as u64);
            stack.push(leaf_count_hi);
            stack.push(leaf_count_lo);

            // Write peaks to memory
            let mut memory: HashMap<BFieldElement, BFieldElement> = HashMap::default();
            rust_shadowing_helper_functions::list::list_insert(
                peaks_pointer,
                mmra.peaks(),
                &mut memory,
            );
            let nondeterminism = NonDeterminism::default().with_ram(memory);

            ProcedureInitialState {
                stack,
                nondeterminism,
                ..Default::default()
            }
        }
    }
}

#[cfg(test)]
mod benches {
    use super::*;
    use crate::traits::procedure::ShadowedProcedure;
    use crate::traits::rust_shadow::RustShadow;

    #[test]
    fn verify_from_secret_in_benchmark() {
        ShadowedProcedure::new(MmrVerifyFromSecretInLeafIndexOnStack).bench();
    }
}
