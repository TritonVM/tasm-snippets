use ndarray::Array1;
use triton_vm::air::memory_layout::DynamicTasmConstraintEvaluationMemoryLayout;
use triton_vm::air::memory_layout::IntegralMemoryLayout;
use triton_vm::air::memory_layout::StaticTasmConstraintEvaluationMemoryLayout;
use triton_vm::air::tasm_air_constraints::dynamic_air_constraint_evaluation_tasm;
use triton_vm::air::tasm_air_constraints::static_air_constraint_evaluation_tasm;
use triton_vm::prelude::*;
use triton_vm::table::challenges::Challenges;
use triton_vm::table::extension_table::Evaluable;
use triton_vm::table::extension_table::Quotientable;
use triton_vm::table::master_table::MasterExtTable;
use triton_vm::twenty_first::math::x_field_element::EXTENSION_DEGREE;

use crate::data_type::ArrayType;
use crate::data_type::DataType;
use crate::library::Library;
use crate::traits::basic_snippet::BasicSnippet;
use crate::triton_vm::table::*;
use crate::verifier::challenges::shared::conventional_challenges_pointer;

#[derive(Debug, Clone, Copy)]
pub enum MemoryLayout {
    Dynamic(DynamicTasmConstraintEvaluationMemoryLayout),
    Static(StaticTasmConstraintEvaluationMemoryLayout),
}

impl MemoryLayout {
    pub fn conventional_static() -> Self {
        const CURRENT_BASE_ROW_PTR: u64 = 30u64;
        const BASE_ROW_SIZE: u64 = (NUM_BASE_COLUMNS * EXTENSION_DEGREE) as u64;
        const EXT_ROW_SIZE: u64 = (NUM_EXT_COLUMNS * EXTENSION_DEGREE) as u64;
        const METADATA_SIZE_PER_PROOF_ITEM_ELEMENT: u64 = 2; // 1 for discriminant, 1 for elem size
        let mem_layout = StaticTasmConstraintEvaluationMemoryLayout {
            free_mem_page_ptr: BFieldElement::new(((1u64 << 32) - 2) * (1u64 << 32)),
            curr_base_row_ptr: BFieldElement::new(CURRENT_BASE_ROW_PTR),
            curr_ext_row_ptr: BFieldElement::new(
                CURRENT_BASE_ROW_PTR + BASE_ROW_SIZE + METADATA_SIZE_PER_PROOF_ITEM_ELEMENT,
            ),
            next_base_row_ptr: BFieldElement::new(
                CURRENT_BASE_ROW_PTR
                    + BASE_ROW_SIZE
                    + EXT_ROW_SIZE
                    + 2 * METADATA_SIZE_PER_PROOF_ITEM_ELEMENT,
            ),
            next_ext_row_ptr: BFieldElement::new(
                CURRENT_BASE_ROW_PTR
                    + 2 * BASE_ROW_SIZE
                    + EXT_ROW_SIZE
                    + 3 * METADATA_SIZE_PER_PROOF_ITEM_ELEMENT,
            ),
            challenges_ptr: conventional_challenges_pointer(),
        };
        assert!(mem_layout.is_integral());

        Self::Static(mem_layout)
    }

    /// Generate a memory layout that allows you to store the proof anywhere in
    /// memory.
    pub fn conventional_dynamic() -> Self {
        let mem_layout = DynamicTasmConstraintEvaluationMemoryLayout {
            free_mem_page_ptr: BFieldElement::new(((1u64 << 32) - 2) * (1u64 << 32)),
            challenges_ptr: conventional_challenges_pointer(),
        };
        assert!(mem_layout.is_integral());

        Self::Dynamic(mem_layout)
    }

    pub fn challenges_pointer(&self) -> BFieldElement {
        match self {
            MemoryLayout::Dynamic(dl) => dl.challenges_ptr,
            MemoryLayout::Static(sl) => sl.challenges_ptr,
        }
    }

    pub fn is_integral(&self) -> bool {
        match self {
            MemoryLayout::Dynamic(dl) => dl.is_integral(),
            MemoryLayout::Static(sl) => sl.is_integral(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AirConstraintEvaluation {
    pub memory_layout: MemoryLayout,
}

impl AirConstraintEvaluation {
    pub fn new_static(static_layout: StaticTasmConstraintEvaluationMemoryLayout) -> Self {
        Self {
            memory_layout: MemoryLayout::Static(static_layout),
        }
    }

    pub fn new_dynamic(static_layout: DynamicTasmConstraintEvaluationMemoryLayout) -> Self {
        Self {
            memory_layout: MemoryLayout::Dynamic(static_layout),
        }
    }

    pub fn with_conventional_static_memory_layout() -> Self {
        Self {
            memory_layout: MemoryLayout::conventional_static(),
        }
    }

    pub fn with_conventional_dynamic_memory_layout() -> Self {
        Self {
            memory_layout: MemoryLayout::conventional_dynamic(),
        }
    }

    pub fn output_type() -> DataType {
        DataType::Array(Box::new(ArrayType {
            element_type: DataType::Xfe,
            length: MasterExtTable::NUM_CONSTRAINTS,
        }))
    }

    /// Return the concatenated AIR-constraint evaluation
    pub fn host_machine_air_constraint_evaluation(
        input_values: AirConstraintSnippetInputs,
    ) -> Vec<XFieldElement> {
        let current_base_row = Array1::from(input_values.current_base_row);
        let current_ext_row = Array1::from(input_values.current_ext_row);
        let next_base_row = Array1::from(input_values.next_base_row);
        let next_ext_row = Array1::from(input_values.next_ext_row);
        let evaluated_initial_constraints = MasterExtTable::evaluate_initial_constraints(
            current_base_row.view(),
            current_ext_row.view(),
            &input_values.challenges,
        );
        let evaluated_consistency_constraints = MasterExtTable::evaluate_consistency_constraints(
            current_base_row.view(),
            current_ext_row.view(),
            &input_values.challenges,
        );
        let evaluated_transition_constraints = MasterExtTable::evaluate_transition_constraints(
            current_base_row.view(),
            current_ext_row.view(),
            next_base_row.view(),
            next_ext_row.view(),
            &input_values.challenges,
        );
        let evaluated_terminal_constraints = MasterExtTable::evaluate_terminal_constraints(
            current_base_row.view(),
            current_ext_row.view(),
            &input_values.challenges,
        );

        [
            evaluated_initial_constraints,
            evaluated_consistency_constraints,
            evaluated_transition_constraints,
            evaluated_terminal_constraints,
        ]
        .concat()
    }
}

impl BasicSnippet for AirConstraintEvaluation {
    fn inputs(&self) -> Vec<(DataType, String)> {
        match self.memory_layout {
            MemoryLayout::Dynamic(_) => vec![
                (DataType::VoidPointer, "*curr_base_row".to_string()),
                (DataType::VoidPointer, "*curr_extrow".to_string()),
                (DataType::VoidPointer, "*next_base_row".to_string()),
                (DataType::VoidPointer, "*next_ext_row".to_string()),
            ],
            MemoryLayout::Static(_) => vec![],
        }
    }

    fn outputs(&self) -> Vec<(DataType, String)> {
        vec![(Self::output_type(), "evaluated_constraints".to_owned())]
    }

    fn entrypoint(&self) -> String {
        assert!(
            self.memory_layout.is_integral(),
            "Memory layout for input values for constraint evaluation must be integral"
        );

        // Consider parameterizing this entrypoint name if you need more than one instance.
        "tasmlib_verifier_master_ext_table_air_constraint_evaluation".to_owned()
    }

    fn code(&self, _library: &mut Library) -> Vec<LabelledInstruction> {
        assert!(
            self.memory_layout.is_integral(),
            "Memory layout for input values for constraint evaluation must be integral"
        );

        let snippet_body = match self.memory_layout {
            MemoryLayout::Dynamic(dynamic_layout) => {
                dynamic_air_constraint_evaluation_tasm(dynamic_layout)
            }
            MemoryLayout::Static(static_layout) => {
                static_air_constraint_evaluation_tasm(static_layout)
            }
        };

        let entrypoint = self.entrypoint();
        let mut code = triton_asm!(
            {entrypoint}:
        );
        code.extend(snippet_body);
        code.extend(triton_asm!(return));

        code
    }
}

/// Please notice that putting the proof into ND memory will *not* result in
/// memory that's compatible with this layout. So this layout will fail to
/// yield a functional STARK verifier program.
#[cfg(test)]
pub fn an_integral_but_profane_static_memory_layout() -> StaticTasmConstraintEvaluationMemoryLayout
{
    let mem_layout = StaticTasmConstraintEvaluationMemoryLayout {
        free_mem_page_ptr: BFieldElement::new((u32::MAX as u64 - 1) * (1u64 << 32)),
        curr_base_row_ptr: BFieldElement::new(1u64),
        curr_ext_row_ptr: BFieldElement::new(1u64 << 20),
        next_base_row_ptr: BFieldElement::new(1u64 << 21),
        next_ext_row_ptr: BFieldElement::new(1u64 << 22),
        challenges_ptr: BFieldElement::new(1u64 << 23),
    };
    assert!(mem_layout.is_integral());

    mem_layout
}

#[cfg(test)]
pub fn an_integral_but_profane_dynamic_memory_layout() -> DynamicTasmConstraintEvaluationMemoryLayout
{
    let mem_layout = DynamicTasmConstraintEvaluationMemoryLayout {
        free_mem_page_ptr: BFieldElement::new((u32::MAX as u64 - 100) * (1u64 << 32)),
        challenges_ptr: BFieldElement::new(1u64 << 30),
    };
    assert!(mem_layout.is_integral());

    mem_layout
}

#[derive(Debug, Clone)]
pub struct AirConstraintSnippetInputs {
    pub current_base_row: Vec<XFieldElement>,
    pub current_ext_row: Vec<XFieldElement>,
    pub next_base_row: Vec<XFieldElement>,
    pub next_ext_row: Vec<XFieldElement>,
    pub challenges: Challenges,
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use arbitrary::Arbitrary;
    use arbitrary::Unstructured;
    use num_traits::ConstZero;
    use rand::distributions::Standard;
    use rand::prelude::*;
    use triton_vm::proof_stream::ProofStream;
    use triton_vm::twenty_first::math::x_field_element::EXTENSION_DEGREE;

    use crate::execute_test;
    use crate::linker::link_for_isolated_run;
    use crate::memory::encode_to_memory;
    use crate::rust_shadowing_helper_functions::array::array_get;
    use crate::rust_shadowing_helper_functions::array::insert_as_array;
    use crate::structure::tasm_object::decode_from_memory_with_size;
    use crate::traits::function::Function;
    use crate::traits::function::FunctionInitialState;

    use super::*;

    #[test]
    fn conventional_air_constraint_memory_layouts_are_integral() {
        assert!(MemoryLayout::conventional_static().is_integral());
        assert!(MemoryLayout::conventional_dynamic().is_integral());
    }

    #[test]
    fn conventional_memory_layout_agrees_with_tvm_proof_stored_at_address_zero() {
        let program = triton_program!(halt);
        let claim = Claim::about_program(&program);

        let proof = triton_vm::prove(
            Stark::default(),
            &claim,
            &program,
            NonDeterminism::default(),
        )
        .unwrap();

        const PROOF_ADDRESS: BFieldElement = BFieldElement::ZERO;
        let mut memory = HashMap::<BFieldElement, BFieldElement>::new();
        let proof_stream = ProofStream::try_from(&proof).unwrap();
        encode_to_memory(&mut memory, PROOF_ADDRESS, proof);

        let assumed_memory_layout = MemoryLayout::conventional_static();
        let MemoryLayout::Static(assumed_memory_layout) = assumed_memory_layout else {
            panic!()
        };
        const BASE_ROW_SIZE: usize = NUM_BASE_COLUMNS * EXTENSION_DEGREE;
        const EXT_ROW_SIZE: usize = NUM_EXT_COLUMNS * EXTENSION_DEGREE;

        let assumed_curr_base_row: [XFieldElement; NUM_BASE_COLUMNS] =
            *decode_from_memory_with_size(
                &memory,
                assumed_memory_layout.curr_base_row_ptr,
                BASE_ROW_SIZE,
            )
            .unwrap();
        let actual_curr_base_row_from_proof = proof_stream.items[4]
            .clone()
            .try_into_out_of_domain_base_row()
            .unwrap();
        assert_eq!(*actual_curr_base_row_from_proof, assumed_curr_base_row);

        let assumed_curr_ext_row: [XFieldElement; NUM_EXT_COLUMNS] = *decode_from_memory_with_size(
            &memory,
            assumed_memory_layout.curr_ext_row_ptr,
            EXT_ROW_SIZE,
        )
        .unwrap();
        let actual_curr_ext_row_from_proof = proof_stream.items[5]
            .clone()
            .try_into_out_of_domain_ext_row()
            .unwrap();
        assert_eq!(*actual_curr_ext_row_from_proof, assumed_curr_ext_row);

        let assumed_next_base_row: [XFieldElement; NUM_BASE_COLUMNS] =
            *decode_from_memory_with_size(
                &memory,
                assumed_memory_layout.next_base_row_ptr,
                BASE_ROW_SIZE,
            )
            .unwrap();
        let actual_next_base_row_from_proof = proof_stream.items[6]
            .clone()
            .try_into_out_of_domain_base_row()
            .unwrap();
        assert_eq!(*actual_next_base_row_from_proof, assumed_next_base_row);

        let assumed_next_ext_row: [XFieldElement; NUM_EXT_COLUMNS] = *decode_from_memory_with_size(
            &memory,
            assumed_memory_layout.next_ext_row_ptr,
            EXT_ROW_SIZE,
        )
        .unwrap();
        let actual_next_ext_row_from_proof = proof_stream.items[7]
            .clone()
            .try_into_out_of_domain_ext_row()
            .unwrap();
        assert_eq!(*actual_next_ext_row_from_proof, assumed_next_ext_row);
    }

    impl Function for AirConstraintEvaluation {
        fn rust_shadow(
            &self,
            _stack: &mut Vec<BFieldElement>,
            _memory: &mut HashMap<BFieldElement, BFieldElement>,
        ) {
            // Never called as we do a more manual test.
            // The more manual test is done bc we don't want to
            // have to simulate all the intermediate calculations
            // that are stored to memory.
            unimplemented!()
        }

        fn pseudorandom_initial_state(
            &self,
            seed: [u8; 32],
            _bench_case: Option<crate::snippet_bencher::BenchmarkCase>,
        ) -> FunctionInitialState {
            // Used for benchmarking
            let mut rng: StdRng = SeedableRng::from_seed(seed);
            let input_values = Self::random_input_values(&mut rng);
            let (memory, stack) = self.prepare_tvm_memory_and_stack(input_values);

            FunctionInitialState { stack, memory }
        }
    }

    #[test]
    fn constraint_evaluation_test() {
        let static_snippet = AirConstraintEvaluation {
            memory_layout: MemoryLayout::Static(an_integral_but_profane_static_memory_layout()),
        };

        let dynamic_snippet = AirConstraintEvaluation {
            memory_layout: MemoryLayout::Dynamic(an_integral_but_profane_dynamic_memory_layout()),
        };

        let mut seed: [u8; 32] = [0u8; 32];
        thread_rng().fill_bytes(&mut seed);
        static_snippet.test_equivalence_with_host_machine_evaluation(seed);

        thread_rng().fill_bytes(&mut seed);
        dynamic_snippet.test_equivalence_with_host_machine_evaluation(seed);
    }

    impl AirConstraintEvaluation {
        fn test_equivalence_with_host_machine_evaluation(&self, seed: [u8; 32]) {
            let mut rng: StdRng = SeedableRng::from_seed(seed);
            let input_values = Self::random_input_values(&mut rng);

            let (tasm_result, _) = self.tasm_result(input_values.clone());
            let host_machine_result = Self::host_machine_air_constraint_evaluation(input_values);

            assert_eq!(tasm_result.len(), host_machine_result.len());
            assert_eq!(
                tasm_result.iter().copied().sum::<XFieldElement>(),
                host_machine_result.iter().copied().sum::<XFieldElement>()
            );
            assert_eq!(tasm_result, host_machine_result);
        }

        pub(crate) fn random_input_values(rng: &mut StdRng) -> AirConstraintSnippetInputs {
            let current_base_row: Vec<XFieldElement> =
                rng.sample_iter(Standard).take(NUM_BASE_COLUMNS).collect();
            let current_ext_row: Vec<XFieldElement> =
                rng.sample_iter(Standard).take(NUM_EXT_COLUMNS).collect();
            let next_base_row: Vec<XFieldElement> =
                rng.sample_iter(Standard).take(NUM_BASE_COLUMNS).collect();
            let next_ext_row: Vec<XFieldElement> =
                rng.sample_iter(Standard).take(NUM_EXT_COLUMNS).collect();

            let mut ch_seed = [0u8; 12000];
            rng.fill_bytes(&mut ch_seed);
            let mut unstructured = Unstructured::new(&ch_seed);
            let challenges: Challenges = Challenges::arbitrary(&mut unstructured).unwrap();

            AirConstraintSnippetInputs {
                current_base_row,
                current_ext_row,
                next_base_row,
                next_ext_row,
                challenges,
            }
        }

        pub(crate) fn prepare_tvm_memory_and_stack(
            &self,
            input_values: AirConstraintSnippetInputs,
        ) -> (HashMap<BFieldElement, BFieldElement>, Vec<BFieldElement>) {
            match self.memory_layout {
                MemoryLayout::Static(static_layout) => {
                    let mut memory: HashMap<BFieldElement, BFieldElement> = HashMap::default();
                    insert_as_array(
                        static_layout.curr_base_row_ptr,
                        &mut memory,
                        input_values.current_base_row,
                    );
                    insert_as_array(
                        static_layout.curr_ext_row_ptr,
                        &mut memory,
                        input_values.current_ext_row,
                    );
                    insert_as_array(
                        static_layout.next_base_row_ptr,
                        &mut memory,
                        input_values.next_base_row,
                    );
                    insert_as_array(
                        static_layout.next_ext_row_ptr,
                        &mut memory,
                        input_values.next_ext_row,
                    );
                    insert_as_array(
                        static_layout.challenges_ptr,
                        &mut memory,
                        input_values.challenges.challenges.to_vec(),
                    );

                    (memory, self.init_stack_for_isolated_run())
                }
                MemoryLayout::Dynamic(dynamic_layout) => {
                    let mut memory: HashMap<BFieldElement, BFieldElement> = HashMap::default();
                    let curr_base_row_ptr = dynamic_layout.challenges_ptr + bfe!(10000);
                    let curr_ext_row_ptr = curr_base_row_ptr
                        + bfe!((input_values.current_base_row.len() * EXTENSION_DEGREE + 1) as u64);
                    let next_base_row_ptr = curr_ext_row_ptr
                        + bfe!((input_values.current_ext_row.len() * EXTENSION_DEGREE + 2) as u64);
                    let next_ext_row_ptr = next_base_row_ptr
                        + bfe!((input_values.next_base_row.len() * EXTENSION_DEGREE + 3) as u64);

                    insert_as_array(
                        curr_base_row_ptr,
                        &mut memory,
                        input_values.current_base_row,
                    );
                    insert_as_array(curr_ext_row_ptr, &mut memory, input_values.current_ext_row);
                    insert_as_array(next_base_row_ptr, &mut memory, input_values.next_base_row);
                    insert_as_array(next_ext_row_ptr, &mut memory, input_values.next_ext_row);
                    insert_as_array(
                        dynamic_layout.challenges_ptr,
                        &mut memory,
                        input_values.challenges.challenges.to_vec(),
                    );

                    let mut stack = self.init_stack_for_isolated_run();
                    stack.push(curr_base_row_ptr);
                    stack.push(curr_ext_row_ptr);
                    stack.push(next_base_row_ptr);
                    stack.push(next_ext_row_ptr);

                    (memory, stack)
                }
            }
        }

        /// Return the pointed-to array and its address.
        /// Note that the result lives as an array in TVM memory but is represented as a list here
        /// since its length is not known at `tasm-lib`'s compile time.
        pub(crate) fn read_result_from_memory(
            mut final_state: VMState,
        ) -> (Vec<XFieldElement>, BFieldElement) {
            let result_pointer = final_state.op_stack.stack.pop().unwrap();
            let mut tasm_result: Vec<XFieldElement> = vec![];
            for i in 0..MasterExtTable::NUM_CONSTRAINTS {
                tasm_result.push(XFieldElement::new(
                    array_get(result_pointer, i, &final_state.ram, EXTENSION_DEGREE)
                        .try_into()
                        .unwrap(),
                ));
            }

            (tasm_result, result_pointer)
        }

        /// Return evaluated constraints and their location in memory
        pub(crate) fn tasm_result(
            &self,
            input_values: AirConstraintSnippetInputs,
        ) -> (Vec<XFieldElement>, BFieldElement) {
            let (init_memory, stack) = self.prepare_tvm_memory_and_stack(input_values);

            let code = link_for_isolated_run(Rc::new(RefCell::new(self.to_owned())));
            let final_state = execute_test(
                &code,
                &mut stack.clone(),
                self.stack_diff(),
                vec![],
                NonDeterminism::default().with_ram(init_memory),
                None,
            );

            Self::read_result_from_memory(final_state)
        }
    }
}

#[cfg(test)]
mod bench {
    use crate::traits::function::ShadowedFunction;
    use crate::traits::rust_shadow::RustShadow;

    use super::*;

    #[test]
    fn bench_air_constraint_evaluation() {
        ShadowedFunction::new(AirConstraintEvaluation {
            memory_layout: MemoryLayout::Static(an_integral_but_profane_static_memory_layout()),
        })
        .bench();
    }
}
