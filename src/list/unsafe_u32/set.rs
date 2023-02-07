use std::collections::HashMap;

use rand::{random, thread_rng, Rng};
use twenty_first::shared_math::b_field_element::BFieldElement;
use twenty_first::shared_math::other::random_elements;

use crate::library::Library;
use crate::rust_shadowing_helper_functions::unsafe_insert_random_list;
use crate::snippet::{DataType, Snippet};
use crate::{get_init_tvm_stack, rust_shadowing_helper_functions, ExecutionState};

#[derive(Clone)]
pub struct Set(pub DataType);

impl Snippet for Set {
    fn inputs(&self) -> Vec<String> {
        // See: https://github.com/TritonVM/tasm-snippets/issues/13
        // _ elem{{N - 1}}, elem{{N - 2}}, ..., elem{{0}} *list index
        vec![
            vec!["element".to_string(); self.0.get_size()],
            vec!["*list".to_string(), "index".to_string()],
        ]
        .concat()
    }

    fn outputs(&self) -> Vec<String> {
        vec![]
    }

    fn input_types(&self) -> Vec<crate::snippet::DataType> {
        vec![
            self.0.clone(),
            DataType::List(Box::new(self.0.clone())),
            DataType::U32,
        ]
    }

    fn output_types(&self) -> Vec<crate::snippet::DataType> {
        vec![]
    }

    fn crash_conditions() -> Vec<String> {
        vec![]
    }

    fn gen_input_states(&self) -> Vec<ExecutionState> {
        fn prepare_state(data_type: &DataType) -> ExecutionState {
            let list_length: usize = thread_rng().gen_range(1..100);
            let index: usize = thread_rng().gen_range(0..list_length);
            let mut stack = get_init_tvm_stack();
            let mut push_value: Vec<BFieldElement> = random_elements(data_type.get_size());
            while let Some(element) = push_value.pop() {
                stack.push(element);
            }

            let list_pointer: BFieldElement = random();
            stack.push(list_pointer);
            stack.push(BFieldElement::new(index as u64));

            let mut memory = HashMap::default();
            unsafe_insert_random_list(list_pointer, list_length, &mut memory, data_type.get_size());
            ExecutionState::with_stack_and_memory(stack, memory, 0)
        }

        vec![
            prepare_state(&self.0),
            prepare_state(&self.0),
            prepare_state(&self.0),
        ]
    }

    fn stack_diff(&self) -> isize {
        -2 - self.0.get_size() as isize
    }

    fn entrypoint(&self) -> String {
        "list_set_element".to_string()
    }

    fn function_body(&self, _library: &mut Library) -> String {
        let entrypoint = self.entrypoint();
        let element_size = self.0.get_size();

        let mut write_elements_to_memory_code = String::default();
        for i in 0..element_size {
            write_elements_to_memory_code.push_str("swap1\n");
            write_elements_to_memory_code.push_str("write_mem\n");
            write_elements_to_memory_code.push_str("pop\n");
            if i != element_size - 1 {
                // Prepare for next write. Not needed for last iteration.
                write_elements_to_memory_code.push_str("push 1\n");
                write_elements_to_memory_code.push_str("add\n");
            }
        }

        format!(
            "
                // BEFORE: _ elem{{N - 1}}, elem{{N - 2}}, ..., elem{{0}} *list index
                // AFTER: _
                {entrypoint}:
                    push {element_size}
                    mul
                    push 1
                    add
                    add

                    // stack: _ elem{{N - 1}}, elem{{N - 2}}, ..., elem{{0}} *list + offset

                    {write_elements_to_memory_code}
                    // stack: _ *list + offset
                    pop

                    return
                    "
        )
    }

    fn rust_shadowing(
        &self,
        stack: &mut Vec<BFieldElement>,
        _std_in: Vec<BFieldElement>,
        _secret_in: Vec<BFieldElement>,
        memory: &mut HashMap<BFieldElement, BFieldElement>,
    ) {
        let index: u32 = stack.pop().unwrap().try_into().unwrap();
        let list_pointer = stack.pop().unwrap();
        let mut element: Vec<BFieldElement> = vec![BFieldElement::new(0); self.0.get_size()];
        for ee in element.iter_mut() {
            *ee = stack.pop().unwrap();
        }
        rust_shadowing_helper_functions::unsafe_list_set(
            list_pointer,
            index as usize,
            element,
            memory,
            self.0.get_size(),
        );
    }
}

#[cfg(test)]
mod list_set_tests {
    use twenty_first::shared_math::b_field_element::BFieldElement;

    use crate::get_init_tvm_stack;
    use crate::rust_shadowing_helper_functions::unsafe_insert_random_list;
    use crate::test_helpers::{rust_tasm_equivalence_prop, rust_tasm_equivalence_prop_new};

    use super::*;

    #[test]
    fn new_snippet_test() {
        rust_tasm_equivalence_prop_new::<Set>(Set(DataType::Bool));
        rust_tasm_equivalence_prop_new::<Set>(Set(DataType::BFE));
        rust_tasm_equivalence_prop_new::<Set>(Set(DataType::U32));
        rust_tasm_equivalence_prop_new::<Set>(Set(DataType::U64));
        rust_tasm_equivalence_prop_new::<Set>(Set(DataType::XFE));
        rust_tasm_equivalence_prop_new::<Set>(Set(DataType::Digest));
    }

    #[test]
    fn list_u32_n_is_one_set() {
        let list_address = BFieldElement::new(48);
        let insert_value = vec![BFieldElement::new(1337)];
        prop_set(DataType::BFE, list_address, 20, insert_value, 2);
    }

    #[test]
    fn list_u32_n_is_three_set() {
        let list_address = BFieldElement::new(48);
        let insert_value = vec![
            BFieldElement::new(1337),
            BFieldElement::new(1337),
            BFieldElement::new(1337),
        ];
        prop_set(DataType::XFE, list_address, 20, insert_value, 2);
    }

    #[test]
    fn list_u32_n_is_two_set() {
        let list_address = BFieldElement::new(1841);
        let push_value = vec![BFieldElement::new(133700), BFieldElement::new(32)];
        prop_set(DataType::U64, list_address, 20, push_value, 0);
    }

    #[test]
    fn list_u32_n_is_five_set() {
        let list_address = BFieldElement::new(558);
        let push_value = vec![
            BFieldElement::new(133700),
            BFieldElement::new(32),
            BFieldElement::new(133700),
            BFieldElement::new(19990),
            BFieldElement::new(88888888),
        ];
        prop_set(DataType::Digest, list_address, 2313, push_value, 589);
    }

    fn prop_set(
        data_type: DataType,
        list_address: BFieldElement,
        init_list_length: u32,
        push_value: Vec<BFieldElement>,
        index: u32,
    ) {
        let expected_end_stack = vec![get_init_tvm_stack()].concat();
        let mut init_stack = get_init_tvm_stack();

        for i in 0..data_type.get_size() {
            init_stack.push(push_value[data_type.get_size() - 1 - i]);
        }
        init_stack.push(list_address);
        init_stack.push(BFieldElement::new(index as u64));

        let mut vm_memory = HashMap::default();

        // Insert length indicator of list, lives on offset = 0 from `list_address`
        unsafe_insert_random_list(
            list_address,
            init_list_length as usize,
            &mut vm_memory,
            data_type.get_size(),
        );

        let _execution_result = rust_tasm_equivalence_prop::<Set>(
            Set(data_type.clone()),
            &init_stack,
            &[],
            &[],
            &mut vm_memory,
            0,
            Some(&expected_end_stack),
        );

        // Verify that length indicator is unchanged
        assert_eq!(
            BFieldElement::new((init_list_length) as u64),
            vm_memory[&list_address]
        );

        // verify that value was inserted at expected place
        for i in 0..data_type.get_size() {
            assert_eq!(
                push_value[i],
                vm_memory[&BFieldElement::new(
                    list_address.value()
                        + 1
                        + data_type.get_size() as u64 * index as u64
                        + i as u64
                )]
            );
        }
    }
}