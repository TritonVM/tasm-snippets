use triton_vm::prelude::*;
use triton_vm::proof_item::ProofItemVariant;
use triton_vm::twenty_first::math::x_field_element::EXTENSION_DEGREE;

use crate::data_type::DataType;
use crate::hashing::sponge_hasher::pad_and_absorb_all::PadAndAbsorbAll;
use crate::library::Library;
use crate::traits::basic_snippet::BasicSnippet;

const MAX_SIZE_FOR_DYNAMICALLY_SIZED_PROOF_ITEMS: u32 = 1u32 << 22;

/// Reads a proof item of the supplied type from the [`ProofStream`][proof_stream].
/// Crashes Triton VM if the proof item is not of the expected type.
/// Updates an internal pointer to the next proof item.
///
/// [proof_stream]: triton_vm::proof_stream::ProofStream
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DequeueNextAs {
    pub proof_item: ProofItemVariant,
}

impl DequeueNextAs {
    pub fn new(proof_item: ProofItemVariant) -> Self {
        Self { proof_item }
    }

    fn item_name(&self) -> String {
        self.proof_item.to_string().to_lowercase()
    }

    /// ```text
    /// BEFORE: _ *proof_item_size
    /// AFTER:  _ *proof_item_size
    /// ```
    fn fiat_shamir_absorb_snippet(&self, library: &mut Library) -> Vec<LabelledInstruction> {
        if !self.proof_item.include_in_fiat_shamir_heuristic() {
            return vec![];
        }

        let pad_and_absorb_all = library.import(Box::new(PadAndAbsorbAll));
        triton_asm! {
            dup 0           // _ *proof_item_size *proof_item_size
            call {pad_and_absorb_all}
                            // _ *proof_item_size
        }
    }

    /// ```text
    /// BEFORE: _ *proof_item_iter *proof_item_size
    /// AFTER:  _ *proof_item_size
    /// ```
    fn update_proof_item_iter_to_next_proof_item(&self) -> Vec<LabelledInstruction> {
        triton_asm! {
            dup 0               // _ *proof_item_iter *proof_item_size *proof_item_iter
            {&self.advance_proof_item_pointer_to_next_item()}
            hint next_proof_item_list_element_size_pointer = stack[0]
                                // _ *proof_item_iter *proof_item_size *next_proof_item_size

            swap 1 swap 2       // _ *pi_size *next_pi_size *pi_iter
            write_mem 1 pop 1   // _ *pi_size
        }
    }

    /// ```text
    /// BEFORE: _ *proof_item_size
    /// AFTER:  _ *next_proof_item_size
    /// ```
    fn advance_proof_item_pointer_to_next_item(&self) -> Vec<LabelledInstruction> {
        let Some(static_length) = self.proof_item.payload_static_length() else {
            return self.advance_proof_item_pointer_to_next_item_reading_length_from_memory();
        };
        Self::advance_proof_item_pointer_to_next_item_using_static_size(static_length)
    }

    /// ```text
    /// BEFORE: _ *proof_item_size
    /// AFTER:  _ *next_proof_item_size
    /// ```
    fn advance_proof_item_pointer_to_next_item_using_static_size(
        length: usize,
    ) -> Vec<LabelledInstruction> {
        let length_plus_bookkeeping_offset = length + 2;
        triton_asm! {
            push {length_plus_bookkeeping_offset}
            hint proof_item_length_plus_bookkeeping_offset = stack[0]
            add
        }
    }

    /// ```text
    /// BEFORE: _ *proof_item_size
    /// AFTER:  _ *next_proof_item_size
    /// ```
    fn advance_proof_item_pointer_to_next_item_reading_length_from_memory(
        &self,
    ) -> Vec<LabelledInstruction> {
        triton_asm! {
            read_mem 1          // _ proof_item_size (*proof_item_size - 1)
            hint proof_item_length = stack[1]

            // Verify that size does not exceed max allowed size
            push {MAX_SIZE_FOR_DYNAMICALLY_SIZED_PROOF_ITEMS}
            dup 2
            // _ proof_item_size (*proof_item_size - 1) max_size proof_item_size

            lt
            // _ proof_item_size (*proof_item_size - 1) (max_size > proof_item_size)

            assert
            // _ proof_item_size (*proof_item_size - 1)

            push 2 add add      // _ *next_proof_item_size
        }
    }

    /// ```text
    /// BEFORE: _ *proof_item_size
    /// AFTER:  _ *proof_item_payload
    /// ```
    fn advance_list_element_pointer_to_proof_item_payload(&self) -> Vec<LabelledInstruction> {
        let payload_length_indicator_size = match self.proof_item.payload_static_length() {
            Some(_) => 0,
            None => 1,
        };
        let list_item_length_indicator_size = 1;
        let discriminant_size = 1;
        let bookkeeping_offset =
            payload_length_indicator_size + list_item_length_indicator_size + discriminant_size;

        triton_asm! { push {bookkeeping_offset} hint bookkeeping_offset = stack[0] add }
    }

    /// A `BFielcCodec`` encoding of a `Polynomial<T>` may not contain trailing zeros. This is
    /// because it must be easy to read the degree of the polynomial. In order to be consistent with
    /// that rule, we perform this check here, before returning a pointer to the polynomial to the
    /// caller.
    /// ```text
    /// BEFORE:  _ *proof_item_payload
    /// AFTER:  _ *proof_item_payload
    /// ```
    fn verify_last_xfe_is_non_zero_if_payload_is_polynomial(&self) -> Vec<LabelledInstruction> {
        match self.proof_item {
            ProofItemVariant::FriPolynomial => triton_asm!(
                // _ *fri_polynomial

                dup 0
                push 1
                add
                read_mem 1
                pop 1
                // _ *fri_polynomial num_coefficients

                push {EXTENSION_DEGREE}
                mul
                push 1
                add
                // _ *fri_polynomial offset_last_word

                dup 1
                add
                // _ *proof_item_payload *coefficients_last_word

                read_mem {EXTENSION_DEGREE}
                pop 1
                push 0
                eq
                swap 2
                push 0
                eq
                swap 1
                push 0
                eq
                add
                add
                push {EXTENSION_DEGREE}
                eq
                // _ *proof_item_payload (last_coefficient == 0)

                push 0
                eq
                // _ *proof_item_payload (last_coefficient != 0)

                assert
            ),
            _ => triton_asm!(),
        }
    }
}

impl BasicSnippet for DequeueNextAs {
    fn inputs(&self) -> Vec<(DataType, String)> {
        vec![(DataType::VoidPointer, "*proof_item_iter".to_string())]
    }

    fn outputs(&self) -> Vec<(DataType, String)> {
        let payload_pointer_str = format!("*{}_payload", self.item_name());
        vec![(DataType::VoidPointer, payload_pointer_str)]
    }

    fn entrypoint(&self) -> String {
        let proof_item_name = self.item_name();
        format!("tasmlib_verifier_vm_proof_iter_dequeue_next_as_{proof_item_name}")
    }

    /// ```text
    /// BEFORE: _ *proof_item_iter
    /// AFTER:  _ *proof_item_payload
    /// ```
    fn code(&self, library: &mut Library) -> Vec<LabelledInstruction> {
        let final_hint = format!("hint {}_pointer: Pointer = stack[0]", self.item_name());
        triton_asm! {
            {self.entrypoint()}:
            read_mem 1
            hint proof_item_list_element_size_pointer = stack[1]

            push 1 add
            swap 1              // _ *proof_item_iter *proof_item_size

            push 1 add
            hint proof_item_discriminant_pointer = stack[0]

            read_mem 1          // _ *proof_item_iter discriminant *proof_item_size
            hint proof_item_list_element_size_pointer = stack[0]
            hint proof_item_discriminant = stack[1]

            swap 1
            push {self.proof_item.bfield_codec_discriminant()}
            hint expected_proof_item_discriminant = stack[0]

            eq assert           // _ *proof_item_iter *proof_item_size

            {&self.fiat_shamir_absorb_snippet(library)}
                                // _ *proof_item_iter *proof_item_size

            {&self.update_proof_item_iter_to_next_proof_item()}
                                // _ *proof_item_size

            {&self.advance_list_element_pointer_to_proof_item_payload()}
            {final_hint}        // _ *proof_item_payload

            {&self.verify_last_xfe_is_non_zero_if_payload_is_polynomial()}
            {final_hint}        // _ *proof_item_payload

            return
        }
    }
}

#[cfg(test)]
mod test {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use arbitrary::Arbitrary;
    use arbitrary::Unstructured;
    use itertools::Itertools;
    use num_traits::One;
    use num_traits::Zero;
    use proptest_arbitrary_interop::arb;
    use rand::prelude::*;
    use strum::IntoEnumIterator;
    use test_strategy::proptest;
    use triton_vm::proof_item::ProofItem;
    use triton_vm::proof_stream::ProofStream;
    use triton_vm::table::NUM_BASE_COLUMNS;
    use triton_vm::twenty_first::math::polynomial::Polynomial;
    use triton_vm::twenty_first::prelude::Sponge;

    use crate::empty_stack;
    use crate::execute_with_terminal_state;
    use crate::linker::link_for_isolated_run;
    use crate::memory::encode_to_memory;
    use crate::snippet_bencher::BenchmarkCase;
    use crate::structure::tasm_object::decode_from_memory_with_size;
    use crate::test_helpers::test_rust_equivalence_given_complete_state;
    use crate::traits::procedure::Procedure;
    use crate::traits::procedure::ProcedureInitialState;
    use crate::traits::procedure::ShadowedProcedure;

    use super::*;

    #[derive(Debug)]
    struct RustShadowForDequeueNextAs<'a> {
        dequeue_next_as: DequeueNextAs,

        proof_iter_pointer: BFieldElement,

        stack: &'a mut Vec<BFieldElement>,
        memory: &'a mut HashMap<BFieldElement, BFieldElement>,
        sponge: &'a mut Tip5,
    }

    impl<'a> RustShadowForDequeueNextAs<'a> {
        fn execute(&mut self) -> Vec<BFieldElement> {
            self.assert_correct_discriminant();
            self.maybe_alter_fiat_shamir_heuristic_with_proof_item();
            self.update_stack_using_current_proof_item();
            self.update_proof_item_iter();
            self.public_output()
        }

        fn current_proof_item_list_element_size_pointer(&self) -> BFieldElement {
            let &maybe_pointer = self.memory.get(&self.proof_iter_pointer).unwrap();
            maybe_pointer
        }

        fn current_proof_item_list_element_size(&self) -> BFieldElement {
            let Some(item_length) = self.dequeue_next_as.proof_item.payload_static_length() else {
                return self.fetch_current_proof_item_list_element_size_from_memory();
            };
            let discriminant_length = 1;
            BFieldElement::new((item_length + discriminant_length) as u64)
        }

        /// Accounts for the item's discriminant.
        fn fetch_current_proof_item_list_element_size_from_memory(&self) -> BFieldElement {
            let size_pointer = self.current_proof_item_list_element_size_pointer();
            let &element_size = self.memory.get(&size_pointer).unwrap();
            assert!(
                element_size.value() < MAX_SIZE_FOR_DYNAMICALLY_SIZED_PROOF_ITEMS as u64,
                "proof item size may not exceed max allowed size"
            );
            element_size
        }

        fn next_proof_item_list_element_size_pointer(&self) -> BFieldElement {
            self.current_proof_item_list_element_size_pointer()
                + self.current_proof_item_list_element_size()
                + BFieldElement::one()
        }

        fn discriminant_pointer(&self) -> BFieldElement {
            self.current_proof_item_list_element_size_pointer() + BFieldElement::one()
        }

        fn discriminant(&self) -> BFieldElement {
            let &discriminant = self.memory.get(&self.discriminant_pointer()).unwrap();
            discriminant
        }

        fn proof_item_payload_pointer(&self) -> BFieldElement {
            self.discriminant_pointer()
                + BFieldElement::one()
                + self.proof_item_payload_size_indicator_length()
        }

        fn proof_item_payload_size_indicator_length(&self) -> BFieldElement {
            match self.dequeue_next_as.proof_item.payload_static_length() {
                Some(_) => BFieldElement::zero(),
                None => BFieldElement::one(),
            }
        }

        fn assert_correct_discriminant(&self) {
            let expected_discriminant = self.dequeue_next_as.proof_item.bfield_codec_discriminant();
            assert_eq!(expected_discriminant, self.discriminant().value() as usize);
        }

        fn proof_item(&self) -> ProofItem {
            let maybe_item = decode_from_memory_with_size::<ProofItem>(
                self.memory,
                self.discriminant_pointer(),
                self.current_proof_item_list_element_size().value() as usize,
            );
            *maybe_item.unwrap()
        }

        fn maybe_alter_fiat_shamir_heuristic_with_proof_item(&mut self) {
            let proof_item = self.proof_item();
            if proof_item.include_in_fiat_shamir_heuristic() {
                Tip5::pad_and_absorb_all(self.sponge, &proof_item.encode());
            }
        }

        fn update_stack_using_current_proof_item(&mut self) {
            self.stack.push(self.proof_item_payload_pointer());
        }

        fn update_proof_item_iter(&mut self) {
            let next_item_pointer = self.next_proof_item_list_element_size_pointer();
            self.memory
                .insert(self.proof_iter_pointer, next_item_pointer);
        }

        fn public_output(&self) -> Vec<BFieldElement> {
            vec![]
        }
    }

    impl Procedure for DequeueNextAs {
        fn rust_shadow(
            &self,
            stack: &mut Vec<BFieldElement>,
            memory: &mut HashMap<BFieldElement, BFieldElement>,
            _: &NonDeterminism,
            _: &[BFieldElement],
            sponge: &mut Option<Tip5>,
        ) -> Vec<BFieldElement> {
            RustShadowForDequeueNextAs {
                dequeue_next_as: *self,
                proof_iter_pointer: stack.pop().unwrap(),
                stack,
                memory,
                sponge: sponge.as_mut().unwrap(),
            }
            .execute()
        }

        fn pseudorandom_initial_state(
            &self,
            seed: [u8; 32],
            _: Option<BenchmarkCase>,
        ) -> ProcedureInitialState {
            let mut rng = StdRng::from_seed(seed);
            let mut proof_stream = ProofStream::new();
            proof_stream.enqueue(self.pseudorandom_proof_item(rng.gen()));

            let other_item_type = ProofItemVariant::iter().choose(&mut rng).unwrap();
            let other_dequeue_next_as = DequeueNextAs::new(other_item_type);
            proof_stream.enqueue(other_dequeue_next_as.pseudorandom_proof_item(rng.gen()));

            Self::initial_state_from_proof_stream_and_address(proof_stream, rng.gen())
        }
    }

    impl DequeueNextAs {
        fn initial_state_from_proof_stream_and_address(
            proof_stream: ProofStream,
            address: BFieldElement,
        ) -> ProcedureInitialState {
            let mut ram = HashMap::new();
            encode_to_memory(&mut ram, address, proof_stream);

            // uses highly specific knowledge of `BFieldCodec`
            let address_of_first_element = address + BFieldElement::new(2);
            let proof_iter_address = address - BFieldElement::one();
            ram.insert(proof_iter_address, address_of_first_element);

            ProcedureInitialState {
                stack: [empty_stack(), vec![proof_iter_address]].concat(),
                nondeterminism: NonDeterminism::default().with_ram(ram),
                public_input: vec![],
                sponge: Some(Tip5::init()),
            }
        }

        fn pseudorandom_new(seed: [u8; 32]) -> Self {
            let mut unstructured = Unstructured::new(&seed);
            let proof_item: ProofItem = Arbitrary::arbitrary(&mut unstructured).unwrap();
            let proof_item = proof_item.into();
            Self { proof_item }
        }

        /// An item to be `Dequeued`, matching the variant in `self`.
        fn pseudorandom_proof_item(&self, seed: [u8; 32]) -> ProofItem {
            let mut rng = StdRng::from_seed(seed);
            let proof_stream_seed: [u8; 100] = rng.gen();
            let mut unstructured = Unstructured::new(&proof_stream_seed);

            match &self.proof_item {
                ProofItemVariant::MerkleRoot => {
                    ProofItem::MerkleRoot(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::OutOfDomainBaseRow => {
                    ProofItem::OutOfDomainBaseRow(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::OutOfDomainExtRow => {
                    ProofItem::OutOfDomainExtRow(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::OutOfDomainQuotientSegments => {
                    ProofItem::OutOfDomainQuotientSegments(
                        Arbitrary::arbitrary(&mut unstructured).unwrap(),
                    )
                }
                ProofItemVariant::AuthenticationStructure => ProofItem::AuthenticationStructure(
                    Arbitrary::arbitrary(&mut unstructured).unwrap(),
                ),
                ProofItemVariant::MasterBaseTableRows => {
                    ProofItem::MasterBaseTableRows(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::MasterExtTableRows => {
                    ProofItem::MasterExtTableRows(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::Log2PaddedHeight => {
                    ProofItem::Log2PaddedHeight(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::QuotientSegmentsElements => ProofItem::QuotientSegmentsElements(
                    Arbitrary::arbitrary(&mut unstructured).unwrap(),
                ),
                ProofItemVariant::FriCodeword => {
                    ProofItem::FriCodeword(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::FriPolynomial => {
                    ProofItem::FriPolynomial(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
                ProofItemVariant::FriResponse => {
                    ProofItem::FriResponse(Arbitrary::arbitrary(&mut unstructured).unwrap())
                }
            }
        }

        fn test_rust_equivalence(self, initial_state: ProcedureInitialState) {
            test_rust_equivalence_given_complete_state(
                &ShadowedProcedure::new(self),
                &initial_state.stack,
                &initial_state.public_input,
                &initial_state.nondeterminism,
                &initial_state.sponge,
                None,
            );
        }
    }

    #[test]
    fn disallow_too_big_dynamically_sized_proof_item() {
        let dequeue_next_as = DequeueNextAs::new(ProofItemVariant::MasterBaseTableRows);
        let initial_state = initial_state_with_too_big_master_table_rows();
        let code = link_for_isolated_run(Rc::new(RefCell::new(dequeue_next_as)));
        let program = Program::new(&code);
        let tvm_result = execute_with_terminal_state(
            &program,
            &[],
            &initial_state.stack,
            &initial_state.nondeterminism,
            None,
        );

        let rust_result = std::panic::catch_unwind(|| {
            let mut stack = initial_state.stack.clone();
            let mut memory = initial_state.nondeterminism.ram.clone();
            dequeue_next_as.rust_shadow(
                &mut stack,
                &mut memory,
                &NonDeterminism::default(),
                &[],
                &mut None,
            );
        });

        assert!(
            rust_result.is_err() && tvm_result.is_err(),
            "Test case: Too big dynamically-sized proof item must fail on both platforms."
        );
        let err = tvm_result.unwrap_err();
        assert_eq!(InstructionError::AssertionFailed, err);
    }

    fn initial_state_with_too_big_master_table_rows() -> ProcedureInitialState {
        let dummy_master_table_rows = vec![[bfe!(101); NUM_BASE_COLUMNS]; 15000];
        let proof_item = ProofItem::MasterBaseTableRows(dummy_master_table_rows);
        let mut proof_stream = ProofStream::new();
        proof_stream.enqueue(proof_item);
        let address = BFieldElement::zero();
        DequeueNextAs::initial_state_from_proof_stream_and_address(proof_stream, address)
    }

    #[test]
    fn disallow_trailing_zeros_in_xfe_poly_encoding() {
        let dequeue_next_as = DequeueNextAs::new(ProofItemVariant::FriPolynomial);
        let initial_state = initial_state_with_trailing_zeros_in_xfe_poly_encoding();
        let code = link_for_isolated_run(Rc::new(RefCell::new(dequeue_next_as)));
        let program = Program::new(&code);
        let tvm_result = execute_with_terminal_state(
            &program,
            &[],
            &initial_state.stack,
            &initial_state.nondeterminism,
            None,
        );

        let rust_result = std::panic::catch_unwind(|| {
            let mut stack = initial_state.stack.clone();
            let mut memory = initial_state.nondeterminism.ram.clone();
            dequeue_next_as.rust_shadow(
                &mut stack,
                &mut memory,
                &NonDeterminism::default(),
                &[],
                &mut None,
            );
        });

        assert!(
            rust_result.is_err() && tvm_result.is_err(),
            "Test case: Too big dynamically-sized proof item must fail on both platforms."
        );
        let err = tvm_result.unwrap_err();
        assert_eq!(InstructionError::AssertionFailed, err);
    }

    fn initial_state_with_trailing_zeros_in_xfe_poly_encoding() -> ProcedureInitialState {
        let mut encoded_fri_poly_proof_item =
            ProofItem::FriPolynomial(Polynomial::new(vec![xfe!(1), xfe!(2), xfe!(3), xfe!(100)]))
                .encode();

        // Manually set the leading coefficient to zero
        let encoding_length = encoded_fri_poly_proof_item.len();
        for i in 0..EXTENSION_DEGREE {
            encoded_fri_poly_proof_item[encoding_length - 1 - i] = bfe!(0);
        }

        let proof_start_address = BFieldElement::zero();

        let mut ram = HashMap::new();

        let mut address = proof_start_address;

        // Inidicate a field-size of `ProofStream`
        ram.insert(address, bfe!(encoding_length as u64 + 2));
        address.increment();

        // Indicate that there is one proof item
        ram.insert(address, bfe!(1));
        address.increment();

        // Indicate size of encoded proof item
        ram.insert(address, bfe!(encoding_length as u64));
        address.increment();

        // Add proof item to ND memory
        for word in encoded_fri_poly_proof_item {
            ram.insert(address, word);
            address.increment();
        }

        // uses highly specific knowledge of `BFieldCodec`
        let address_of_first_element = proof_start_address + BFieldElement::new(2);
        let proof_iter_address = proof_start_address - BFieldElement::one();
        ram.insert(proof_iter_address, address_of_first_element);
        ProcedureInitialState {
            stack: [empty_stack(), vec![proof_iter_address]].concat(),
            nondeterminism: NonDeterminism::default().with_ram(ram),
            public_input: vec![],
            sponge: Some(Tip5::init()),
        }
    }

    #[test]
    fn dequeueing_a_merkle_root_is_equivalent_in_rust_and_tasm() {
        let dequeue_next_as = DequeueNextAs::new(ProofItemVariant::MerkleRoot);
        dequeue_next_as.test_rust_equivalence(small_merkle_root_initial_state());
    }

    fn small_merkle_root_initial_state() -> ProcedureInitialState {
        let dummy_digest = Digest::new([42, 43, 44, 45, 46].map(BFieldElement::new));
        let proof_item = ProofItem::MerkleRoot(dummy_digest);
        let mut proof_stream = ProofStream::new();
        proof_stream.enqueue(proof_item.clone());
        proof_stream.enqueue(proof_item);

        let address = BFieldElement::zero();
        DequeueNextAs::initial_state_from_proof_stream_and_address(proof_stream, address)
    }

    #[proptest]
    fn dequeuing_as_suggested_element_is_equivalent_in_rust_and_tasm(seed: [u8; 32]) {
        let dequeue_next_as = DequeueNextAs::pseudorandom_new(seed);
        let initial_state = dequeue_next_as.pseudorandom_initial_state(seed, None);
        dequeue_next_as.test_rust_equivalence(initial_state);
    }

    /// Helps testing dequeuing multiple items.
    #[derive(Debug, Default, Clone, Eq, PartialEq)]
    struct TestHelperDequeueMultipleAs {
        proof_items: Vec<ProofItemVariant>,
    }

    impl BasicSnippet for TestHelperDequeueMultipleAs {
        fn inputs(&self) -> Vec<(DataType, String)> {
            vec![(DataType::VoidPointer, "*proof_item_iter".to_string())]
        }

        fn outputs(&self) -> Vec<(DataType, String)> {
            vec![(DataType::VoidPointer, "*proof_item_iter".to_string())]
        }

        fn entrypoint(&self) -> String {
            "test_helper_dequeue_multiple_as".to_string()
        }

        fn code(&self, library: &mut Library) -> Vec<LabelledInstruction> {
            let dequeue_next_as_entrypoints = self
                .proof_items
                .iter()
                .map(|&item| library.import(Box::new(DequeueNextAs::new(item))))
                .collect::<Vec<_>>();

            let code_body = dequeue_next_as_entrypoints
                .into_iter()
                .flat_map(|snippet| triton_asm!(dup 0 call {snippet} pop 1))
                .collect_vec();

            triton_asm!({self.entrypoint()}: {&code_body} return)
        }
    }

    impl Procedure for TestHelperDequeueMultipleAs {
        fn rust_shadow(
            &self,
            stack: &mut Vec<BFieldElement>,
            memory: &mut HashMap<BFieldElement, BFieldElement>,
            non_determinism: &NonDeterminism,
            std_in: &[BFieldElement],
            sponge: &mut Option<Tip5>,
        ) -> Vec<BFieldElement> {
            let &proof_iter_pointer = stack.last().unwrap();
            for &proof_item in &self.proof_items {
                let dequeue_next_as = DequeueNextAs { proof_item };
                stack.push(proof_iter_pointer);
                dequeue_next_as.rust_shadow(stack, memory, non_determinism, std_in, sponge);
                stack.pop().unwrap();
            }
            vec![]
        }

        fn pseudorandom_initial_state(
            &self,
            seed: [u8; 32],
            _: Option<BenchmarkCase>,
        ) -> ProcedureInitialState {
            let mut rng = StdRng::from_seed(seed);
            let mut proof_stream = ProofStream::new();
            for &proof_item in &self.proof_items {
                let dequeue_next_as = DequeueNextAs { proof_item };
                let item = dequeue_next_as.pseudorandom_proof_item(rng.gen());
                proof_stream.enqueue(item);
            }
            DequeueNextAs::initial_state_from_proof_stream_and_address(proof_stream, rng.gen())
        }
    }

    impl TestHelperDequeueMultipleAs {
        fn test_rust_equivalence(self, initial_state: ProcedureInitialState) {
            test_rust_equivalence_given_complete_state(
                &ShadowedProcedure::new(self),
                &initial_state.stack,
                &initial_state.public_input,
                &initial_state.nondeterminism,
                &initial_state.sponge,
                None,
            );
        }
    }

    #[test]
    fn dequeue_two() {
        let proof_items = vec![ProofItemVariant::MerkleRoot, ProofItemVariant::MerkleRoot];
        let dequeue_multiple = TestHelperDequeueMultipleAs { proof_items };
        let initial_state = dequeue_multiple.pseudorandom_initial_state([0; 32], None);
        dequeue_multiple.test_rust_equivalence(initial_state);
    }

    #[proptest]
    fn dequeue_multiple(#[strategy(arb())] proof_items: Vec<ProofItem>, seed: [u8; 32]) {
        let proof_items = proof_items.into_iter().map(|i| i.into()).collect_vec();
        let dequeue_multiple = TestHelperDequeueMultipleAs { proof_items };
        let initial_state = dequeue_multiple.pseudorandom_initial_state(seed, None);
        dequeue_multiple.test_rust_equivalence(initial_state);
    }
}
