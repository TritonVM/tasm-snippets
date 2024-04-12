use crate::data_type::DataType;
use crate::hashing::algebraic_hasher::hash_static_size::HashStaticSize;
use crate::hashing::merkle_verify::MerkleVerify;
use crate::library::Library;
use crate::recufier::fri::verify::FriSnippet;
use crate::traits::basic_snippet::BasicSnippet;
use strum::Display;
use triton_vm::prelude::*;
use triton_vm::table::NUM_BASE_COLUMNS;
use triton_vm::table::NUM_EXT_COLUMNS;
use triton_vm::table::NUM_QUOTIENT_SEGMENTS;
use triton_vm::twenty_first::shared_math::tip5::DIGEST_LENGTH;
use triton_vm::twenty_first::shared_math::x_field_element::EXTENSION_DEGREE;

#[derive(Debug, Copy, Clone, Display)]
pub enum ColumnType {
    Base,
    Extension,
    Quotient,
}

/// Crashes the VM is the base table rows do not authenticate against the provided Merkle root
/// First hashes the rows, then verifies that the digests belong in the Merkle tree.
pub struct VerifyTableRows {
    pub column_type: ColumnType,
}

impl VerifyTableRows {
    pub fn row_size(&self) -> usize {
        match self.column_type {
            ColumnType::Base => NUM_BASE_COLUMNS,
            ColumnType::Extension => NUM_EXT_COLUMNS * EXTENSION_DEGREE,
            ColumnType::Quotient => NUM_QUOTIENT_SEGMENTS * EXTENSION_DEGREE,
        }
    }
}

impl BasicSnippet for VerifyTableRows {
    fn inputs(&self) -> Vec<(DataType, String)> {
        vec![
            (DataType::U32, "num_combination_codeword_checks".to_owned()),
            (DataType::U32, "merkle_tree_height".to_owned()),
            (DataType::VoidPointer, "*merkle_tree_root".to_owned()),
            (
                FriSnippet::indexed_leaves_list_type(),
                "*fri_revealed".to_owned(),
            ),
            // type of {base|ext|quot} table rows i
            // `Vec<[{BaseFieldElement, XFieldElement, XFieldElement}: COLUMN_COUNT]>` but encoded
            // in memory as a flat structure. So I'm not sure what type to use here. Anyway, it's
            // certainly a list.
            (DataType::VoidPointer, "*table_rows".to_owned()),
        ]
    }

    fn outputs(&self) -> Vec<(DataType, String)> {
        vec![]
    }

    fn entrypoint(&self) -> String {
        format!("verify_{}_table_rows", self.column_type)
    }

    fn code(&self, library: &mut Library) -> Vec<LabelledInstruction> {
        let entrypoint = self.entrypoint();

        let hash_row = library.import(Box::new(HashStaticSize {
            size: self.row_size(),
        }));
        let merkle_root_verify = library.import(Box::new(MerkleVerify));

        let loop_label = format!("{entrypoint}_loop");
        let loop_code = triton_asm!(
            // Invariant: _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index *row_elem
            {loop_label}:
                // Check end-loop condition
                dup 4
                push 0
                eq
                skiz
                    return

                // 1. dup up *merkle_tree_root, and get digest
                // 2.
                //   a. Read revealed FRI leaf index
                //   b. Update revealed FRI leaf index to next value
                // 3. get digest through call to `hash_base_row`, and update pointer value to next item
                // 4. dup Merkle tree height to the top.

                /* 1. */
                dup 2
                push {DIGEST_LENGTH - 1}
                add
                read_mem {DIGEST_LENGTH}
                pop 1
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index *row_elem [mt_root]

                /* 2. */
                dup 6
                read_mem 1
                push {EXTENSION_DEGREE + 2}
                add
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index *row_elem [mt_root] fri_revealed_leaf_index (*fri_revealed_leaf_index + 4)
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index *row_elem [mt_root] fri_revealed_leaf_index *fri_revealed_leaf_index_next

                swap 8
                pop 1
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index_next *row_elem [mt_root] fri_revealed_leaf_index

                /* 3. */
                dup 6
                call {hash_row}
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index_next *row_elem [mt_root] fri_revealed_leaf_index [base_row_digest] *row_elem_next

                swap 12
                pop 1
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index_next *row_elem_next [mt_root] fri_revealed_leaf_index [base_row_digest]

                /* 4. */
                dup 14
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index_next *row_elem_next [mt_root] fri_revealed_leaf_index [base_row_digest] merkle_tree_height

                call {merkle_root_verify}
                // _ remaining_iterations merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index_next *row_elem_next

                swap 4
                push -1
                add
                swap 4
                // _ (remaining_iterations - 1) merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index_next *row_elem_next

                recurse
        );

        triton_asm!(
            // _ num_combination_codeword_checks merkle_tree_height *merkle_tree_root *fri_revealed *table_rows
            {entrypoint}:

                swap 1
                push {1 + EXTENSION_DEGREE}
                add
                swap 1
                // _ num_combination_codeword_checks merkle_tree_height *merkle_tree_root (*fri_revealed_first_elem.0) *table_rows

                // Verify length of `rows`
                read_mem 1
                push 2
                add
                swap 1
                // _ num_combination_codeword_checks merkle_tree_height *merkle_tree_root (*fri_revealed_first_elem.0) *table_rows[0] length

                dup 5
                eq
                assert
                // _ num_combination_codeword_checks merkle_tree_height *merkle_tree_root (*fri_revealed_first_elem.0) *table_rows[0]

                call {loop_label}
                // _ 0 merkle_tree_height *merkle_tree_root *fri_revealed_leaf_index_next *row_elem_next

                pop 5
                // _

                return

            {&loop_code}
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use itertools::Itertools;
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use triton_vm::twenty_first::shared_math::other::random_elements;
    use triton_vm::twenty_first::shared_math::tip5::RATE;
    use triton_vm::twenty_first::util_types::algebraic_hasher::AlgebraicHasher;
    use triton_vm::twenty_first::util_types::algebraic_hasher::Sponge;
    use triton_vm::twenty_first::util_types::merkle_tree::CpuParallel;
    use triton_vm::twenty_first::util_types::merkle_tree::MerkleTree;
    use triton_vm::twenty_first::util_types::merkle_tree::MerkleTreeInclusionProof;
    use triton_vm::twenty_first::util_types::merkle_tree_maker::MerkleTreeMaker;

    use crate::memory::encode_to_memory;
    use crate::rust_shadowing_helper_functions::list::list_insert;
    use crate::snippet_bencher::BenchmarkCase;
    use crate::traits::procedure::Procedure;
    use crate::traits::procedure::ProcedureInitialState;
    use crate::traits::procedure::ShadowedProcedure;
    use crate::traits::rust_shadow::RustShadow;
    use crate::VmHasher;

    use super::*;

    #[test]
    fn verify_table_pbt_base() {
        ShadowedProcedure::new(VerifyTableRows {
            column_type: ColumnType::Base,
        })
        .test()
    }

    #[test]
    fn verify_table_pbt_ext() {
        ShadowedProcedure::new(VerifyTableRows {
            column_type: ColumnType::Extension,
        })
        .test()
    }

    #[test]
    fn verify_table_pbt_quot() {
        ShadowedProcedure::new(VerifyTableRows {
            column_type: ColumnType::Quotient,
        })
        .test()
    }

    // TODO: Add negative tests, to verify that VM crashes with bad authentication paths, and
    // that it crashes if fed a leaf index that's not a valid u32.

    impl Procedure for VerifyTableRows {
        fn rust_shadow(
            &self,
            stack: &mut Vec<BFieldElement>,
            memory: &mut HashMap<BFieldElement, BFieldElement>,
            nondeterminism: &NonDeterminism<BFieldElement>,
            _public_input: &[BFieldElement],
            sponge: &mut Option<VmHasher>,
        ) -> Vec<BFieldElement> {
            fn verify_one_row(
                leaf_index: u32,
                merkle_root: Digest,
                merkle_tree_height: u32,
                authentication_path: Vec<Digest>,
                row: &[BFieldElement],
                sponge: &mut VmHasher,
            ) {
                // We define a local hash_varlen to be able to simulate what happens to the sponge,
                // as this is required by the test framework.
                fn local_hash_varlen(input: &[BFieldElement], sponge: &mut VmHasher) -> Digest {
                    *sponge = Tip5::init();
                    sponge.pad_and_absorb_all(input);
                    let produce: [BFieldElement; RATE] = sponge.squeeze();

                    Digest::new((&produce[..DIGEST_LENGTH]).try_into().unwrap())
                }

                let leaf_digest = local_hash_varlen(row, sponge);
                let merkle_tree_inclusion_proof = MerkleTreeInclusionProof::<Tip5> {
                    tree_height: merkle_tree_height as usize,
                    indexed_leaves: vec![(leaf_index as usize, leaf_digest)],
                    authentication_structure: authentication_path,
                    _hasher: std::marker::PhantomData,
                };

                assert!(merkle_tree_inclusion_proof.verify(merkle_root));
            }

            *sponge = Some(Tip5::init());
            let table_rows_pointer = stack.pop().unwrap();
            let fri_revealed_pointer = stack.pop().unwrap();
            let merkle_tree_root_pointer = stack.pop().unwrap();
            let merkle_tree_height: u32 = stack.pop().unwrap().try_into().unwrap();
            let num_combination_codeword_checks: u32 = stack.pop().unwrap().try_into().unwrap();

            let merkle_root = Digest::new(
                (0..DIGEST_LENGTH)
                    .map(|i| memory[&(merkle_tree_root_pointer + bfe!(i as u32))])
                    .collect_vec()
                    .try_into()
                    .unwrap(),
            );

            // Verify all rows
            let mut j = 0;
            for i in 0..num_combination_codeword_checks {
                // Read a row from memory
                let row = (0..self.row_size())
                    .map(|l| {
                        memory[&(table_rows_pointer
                            + bfe!(l as u64 + 1 + (self.row_size() as u64) * i as u64))]
                    })
                    .collect_vec();

                // Read leaf index as provided by the FRI verifier
                let leaf_index: u32 = nondeterminism.ram
                    [&(fri_revealed_pointer + bfe!(4) + BFieldElement::new(i as u64 * 4))]
                    .try_into()
                    .unwrap();
                let mut authentication_path = vec![];
                for _ in 0..merkle_tree_height {
                    authentication_path.push(nondeterminism.digests[j]);
                    j += 1;
                }

                verify_one_row(
                    leaf_index,
                    merkle_root,
                    merkle_tree_height,
                    authentication_path,
                    &row,
                    sponge.as_mut().unwrap(),
                )
            }

            vec![]
        }

        fn pseudorandom_initial_state(
            &self,
            seed: [u8; 32],
            bench_case: Option<crate::snippet_bencher::BenchmarkCase>,
        ) -> ProcedureInitialState {
            let mut rng: StdRng = SeedableRng::from_seed(seed);
            let merkle_tree_height = match bench_case {
                Some(BenchmarkCase::CommonCase) => 10,
                Some(BenchmarkCase::WorstCase) => 15,
                None => rng.gen_range(2..7),
            };
            let num_leafs = 1 << merkle_tree_height;
            let num_combination_codeword_checks = 3;
            let mut memory = HashMap::default();

            let rows: Vec<Vec<BFieldElement>> =
                vec![vec![rng.gen(); self.row_size()]; num_combination_codeword_checks];
            let leaf_indices: Vec<usize> = (0..num_combination_codeword_checks)
                .map(|_| rng.gen_range(0..num_leafs))
                .collect_vec();

            // Construct Merkle tree with specified rows as preimages to the leafs at the specified
            // indices.
            let mut leafs: Vec<Digest> = random_elements(1 << merkle_tree_height);
            for (leaf_index, leaf_preimage) in leaf_indices.iter().zip_eq(rows.iter()) {
                leafs[*leaf_index] = Tip5::hash_varlen(leaf_preimage);
            }

            let merkle_tree: MerkleTree<Tip5> = CpuParallel::from_digests(&leafs).unwrap();
            let merkle_root = merkle_tree.root();
            let merkle_root_pointer: BFieldElement = rng.gen();
            encode_to_memory(&mut memory, merkle_root_pointer, merkle_root);

            // Insert all rows into memory, as a list
            let row_pointer: BFieldElement = rng.gen();
            memory.insert(row_pointer, bfe!(num_combination_codeword_checks as u64));
            let mut j: BFieldElement = bfe!(1);
            for row in rows {
                for word in row {
                    memory.insert(row_pointer + j, word);
                    j.increment();
                }
            }

            let mocked_fri_return_value: Vec<(u32, XFieldElement)> = leaf_indices
                .iter()
                .map(|x| *x as u32)
                .zip((0..num_combination_codeword_checks).map(|_| rng.gen()))
                .collect_vec();
            let fri_return_value_pointer = rng.gen();
            list_insert(
                fri_return_value_pointer,
                mocked_fri_return_value,
                &mut memory,
            );

            let mut authentication_paths: Vec<Digest> = vec![];
            for leaf_index in leaf_indices {
                authentication_paths
                    .extend(merkle_tree.authentication_structure(&[leaf_index]).unwrap());
            }

            let stack = [
                self.init_stack_for_isolated_run(),
                vec![
                    bfe!(num_combination_codeword_checks as u64),
                    bfe!(merkle_tree_height),
                    merkle_root_pointer,
                    fri_return_value_pointer,
                    row_pointer,
                ],
            ]
            .concat();
            ProcedureInitialState {
                stack,
                nondeterminism: NonDeterminism::default()
                    .with_ram(memory)
                    .with_digests(authentication_paths),
                public_input: vec![],
                sponge: None,
            }
        }
    }
}
