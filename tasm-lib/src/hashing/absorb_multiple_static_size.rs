use std::collections::HashMap;

use itertools::Itertools;
use rand::rngs::StdRng;
use rand::Rng;
use rand::RngCore;
use rand::SeedableRng;
use triton_vm::prelude::*;
use triton_vm::twenty_first::math::tip5::RATE;
use triton_vm::twenty_first::prelude::Sponge;

use crate::data_type::DataType;
use crate::memory::load_words_from_memory_pop_pointer;
use crate::snippet_bencher::BenchmarkCase;
use crate::traits::basic_snippet::BasicSnippet;
use crate::traits::procedure::Procedure;
use crate::traits::procedure::ProcedureInitialState;
use crate::VmHasher;

/// Absorb a sequence of field elements stored in memory, into the Sponge.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct AbsorbMultipleStaticSize {
    pub size: usize,
}

impl AbsorbMultipleStaticSize {
    fn padded_length(&self) -> usize {
        (self.size + 1).next_multiple_of(RATE)
    }

    fn num_absorbs_before_pad(&self) -> usize {
        self.padded_length() / RATE - 1
    }

    fn num_remaining_words(&self) -> usize {
        self.size - self.num_absorbs_before_pad() * RATE
    }

    fn read_remainder_and_pad(&self) -> Vec<LabelledInstruction> {
        let num_zeros = self.padded_length() - self.size - 1;
        let adjust_read_pointer = match self.num_remaining_words() {
            0 | 1 => triton_asm!(),
            n => triton_asm!(
                // _ *first_unread_word
                push {n-1}
                add
                // _ *last_unread_word
            ),
        };
        [
            vec![triton_asm!(push 0); num_zeros].concat(),
            triton_asm!(push 1),
            triton_asm!(dup {num_zeros + 1}),
            // _ *first_unread_word [0] 1 *first_unread_word
            adjust_read_pointer,
            // _ *first_unread_word [0] 1 *last_unread_word
            load_words_from_memory_pop_pointer(self.num_remaining_words()),
            // _ *first_unread_word [0] 1 [un-absorbed-words]
        ]
        .concat()
    }
}

impl BasicSnippet for AbsorbMultipleStaticSize {
    fn inputs(&self) -> Vec<(DataType, String)> {
        vec![(DataType::VoidPointer, "*sequence".to_string())]
    }

    fn outputs(&self) -> Vec<(DataType, String)> {
        vec![(DataType::VoidPointer, "(sequence + size)".to_string())]
    }

    fn entrypoint(&self) -> String {
        format!("tasmlib_hashing_absorb_multiple_static_size_{}", self.size)
    }

    fn code(&self, _library: &mut crate::library::Library) -> Vec<LabelledInstruction> {
        let entrypoint = self.entrypoint();
        let absorb_once = triton_asm!(
            // _ d c b a *address
            sponge_absorb_mem // _ h g f e (*address + RATE)
        );

        let absorb_all_non_padded = vec![absorb_once; self.num_absorbs_before_pad()].concat();
        let read_remainder_and_pad = self.read_remainder_and_pad();

        triton_asm! {
            // BEFORE: _ *bfe_sequence
            // AFTER:  _
            {entrypoint}:
                // _ *bfe_sequence

                push 0
                push 0
                push 0
                push 0
                swap 4
                // _ 0 0 0 0 *bfe_sequence

                {&absorb_all_non_padded}
                // _ g0 g1 g2 g3 *next_unread_word

                swap 4
                pop 4
                // _ *next_unread_word

                {&read_remainder_and_pad}

                sponge_absorb
                push {self.size % RATE}
                add

                // _ (*address + self.size)

                return
        }
    }
}

impl Procedure for AbsorbMultipleStaticSize {
    fn rust_shadow(
        &self,
        stack: &mut Vec<BFieldElement>,
        memory: &mut HashMap<BFieldElement, BFieldElement>,
        _nondeterminism: &NonDeterminism,
        _public_input: &[BFieldElement],
        sponge: &mut Option<VmHasher>,
    ) -> Vec<BFieldElement> {
        // read arguments
        let address = stack.pop().unwrap();

        // read sequence from memory
        let mut sequence = vec![];
        for i in 0..self.size {
            sequence.push(
                memory
                    .get(&(address + BFieldElement::new(i as u64)))
                    .copied()
                    .unwrap(),
            )
        }

        let sponge = sponge.as_mut().expect("sponge must be initialized");
        sponge.pad_and_absorb_all(&sequence);

        stack.push(address + BFieldElement::new(self.size.try_into().unwrap()));

        // output empty
        vec![]
    }

    fn pseudorandom_initial_state(
        &self,
        seed: [u8; 32],
        _bench_case: Option<BenchmarkCase>,
    ) -> ProcedureInitialState {
        let mut rng: StdRng = SeedableRng::from_seed(seed);

        // sample address
        let address = BFieldElement::new(rng.next_u64() % (1 << 20));

        let sequence = (0..self.size)
            .map(|_| rng.gen::<BFieldElement>())
            .collect_vec();

        // write to memory
        let mut memory = HashMap::new();
        for (i, s) in sequence.into_iter().enumerate() {
            memory.insert(address + BFieldElement::new(i as u64), s);
        }
        let nondeterminism = NonDeterminism::default().with_ram(memory);

        let stack = [self.init_stack_for_isolated_run(), vec![address]].concat();

        let vm_hasher_state = Tip5 { state: rng.gen() };

        ProcedureInitialState {
            stack,
            nondeterminism,
            public_input: vec![],
            sponge: Some(vm_hasher_state),
        }
    }
}

#[cfg(test)]
mod test {
    use proptest_arbitrary_interop::arb;
    use test_strategy::proptest;

    use crate::traits::procedure::ShadowedProcedure;
    use crate::traits::rust_shadow::RustShadow;

    use super::AbsorbMultipleStaticSize;

    #[test]
    fn absorb_multiple_static_size_0() {
        ShadowedProcedure::new(AbsorbMultipleStaticSize { size: 0 }).test();
    }

    #[test]
    fn absorb_multiple_static_size_1() {
        ShadowedProcedure::new(AbsorbMultipleStaticSize { size: 1 }).test();
    }

    #[test]
    fn absorb_multiple_static_size_9() {
        ShadowedProcedure::new(AbsorbMultipleStaticSize { size: 9 }).test();
    }

    #[test]
    fn absorb_multiple_static_size_small_pbt() {
        for size in 0..30 {
            println!("Testing size {size}");
            ShadowedProcedure::new(AbsorbMultipleStaticSize { size }).test();
        }
    }

    #[proptest(cases = 40)]
    fn absorb_multiple_static_size_pbt_pbt(#[strategy(arb())] size: u8) {
        ShadowedProcedure::new(AbsorbMultipleStaticSize {
            size: size as usize,
        })
        .test();
    }
}

#[cfg(test)]
mod benches {
    use crate::traits::procedure::ShadowedProcedure;
    use crate::traits::rust_shadow::RustShadow;

    use super::AbsorbMultipleStaticSize;

    #[test]
    fn absorb_multiple_static_size_benchmark_102() {
        ShadowedProcedure::new(AbsorbMultipleStaticSize { size: 102 }).bench();
    }

    #[test]
    fn absorb_multiple_static_size_benchmark_400() {
        ShadowedProcedure::new(AbsorbMultipleStaticSize { size: 400 }).bench();
    }

    #[test]
    fn absorb_multiple_static_size_benchmark_2002() {
        ShadowedProcedure::new(AbsorbMultipleStaticSize { size: 2002 }).bench();
    }
}
