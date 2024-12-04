use num_traits::Zero;
use rand::prelude::*;
use triton_vm::prelude::*;
use twenty_first::math::traits::PrimitiveRootOfUnity as PRU;

use crate::data_type::DataType;
use crate::empty_stack;
use crate::snippet_bencher::BenchmarkCase;
use crate::traits::basic_snippet::BasicSnippet;
use crate::traits::closure::Closure;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct PrimitiveRootOfUnity;

impl BasicSnippet for PrimitiveRootOfUnity {
    fn inputs(&self) -> Vec<(DataType, String)> {
        vec![(DataType::U64, "order".to_owned())]
    }

    fn outputs(&self) -> Vec<(DataType, String)> {
        vec![(DataType::Bfe, "root_of_unity".to_string())]
    }

    fn entrypoint(&self) -> String {
        "tasmlib_arithmetic_bfe_primitive_root_of_unity".to_string()
    }

    fn code(&self, _library: &mut crate::library::Library) -> Vec<LabelledInstruction> {
        let entrypoint = self.entrypoint();

        triton_asm!(
            {entrypoint}:
            // _ order_hi order_lo

            // First check if order i $1^{32}$.

            dup 1
            push 1
            eq
            // _ order_hi order_lo (order_hi == 1)

            dup 1
            push 0
            eq
            mul
            // _ order_hi order_lo (order_hi == 1 && order_lo == 0)

            skiz
                push 1753635133440165772
            // _ order_hi order_lo [root]

            // at this point `st1` *must* be zero.

            dup 1
            push 0
            eq
            assert error_id 140

            // Now we only have to check `order_lo`. We can ignore `order_hi` as we've
            // verified that it's 0 in case the order was not $1^{32}$.

            dup 0 push 1             eq skiz push 1
            dup 0 push {1_u32 << 1}  eq skiz push 18446744069414584320
            dup 0 push {1_u32 << 2}  eq skiz push 281474976710656
            dup 0 push {1_u32 << 3}  eq skiz push 18446744069397807105
            dup 0 push {1_u32 << 4}  eq skiz push 17293822564807737345
            dup 0 push {1_u32 << 5}  eq skiz push 70368744161280
            dup 0 push {1_u32 << 6}  eq skiz push 549755813888
            dup 0 push {1_u32 << 7}  eq skiz push 17870292113338400769
            dup 0 push {1_u32 << 8}  eq skiz push 13797081185216407910
            dup 0 push {1_u32 << 9}  eq skiz push 1803076106186727246
            dup 0 push {1_u32 << 10} eq skiz push 11353340290879379826
            dup 0 push {1_u32 << 11} eq skiz push 455906449640507599
            dup 0 push {1_u32 << 12} eq skiz push 17492915097719143606
            dup 0 push {1_u32 << 13} eq skiz push 1532612707718625687
            dup 0 push {1_u32 << 14} eq skiz push 16207902636198568418
            dup 0 push {1_u32 << 15} eq skiz push 17776499369601055404
            dup 0 push {1_u32 << 16} eq skiz push 6115771955107415310
            dup 0 push {1_u32 << 17} eq skiz push 12380578893860276750
            dup 0 push {1_u32 << 18} eq skiz push 9306717745644682924
            dup 0 push {1_u32 << 19} eq skiz push 18146160046829613826
            dup 0 push {1_u32 << 20} eq skiz push 3511170319078647661
            dup 0 push {1_u32 << 21} eq skiz push 17654865857378133588
            dup 0 push {1_u32 << 22} eq skiz push 5416168637041100469
            dup 0 push {1_u32 << 23} eq skiz push 16905767614792059275
            dup 0 push {1_u32 << 24} eq skiz push 9713644485405565297
            dup 0 push {1_u32 << 25} eq skiz push 5456943929260765144
            dup 0 push {1_u32 << 26} eq skiz push 17096174751763063430
            dup 0 push {1_u32 << 27} eq skiz push 1213594585890690845
            dup 0 push {1_u32 << 28} eq skiz push 6414415596519834757
            dup 0 push {1_u32 << 29} eq skiz push 16116352524544190054
            dup 0 push {1_u32 << 30} eq skiz push 9123114210336311365
            dup 0 push {1_u32 << 31} eq skiz push 4614640910117430873

            // Since all roots happen to be larger than `u32::MAX`, or `1` we can
            // test if the top element is a root or not. If this assumption
            // were to change, VM execution would crash here, and tests would
            // catch that.

            // stack if result found:     _ order_hi order_lo root
            // stack if result not found: _ order_hi order_lo

            dup 0
            push 1
            eq
            // stack if result found:     _ order_hi order_lo root (root == 1)
            // stack if result not found: _ order_hi order_lo (order_lo == 1)

            dup 1
            split
            // Result found:     _ order_hi order_lo root (root == 1) root_hi root_lo
            // Result not found: _ order_hi order_lo (order_lo == 1) 0 order_lo

            pop 1
            // Result found:     _ order_hi order_lo root (root == 1) root_hi
            // Result not found: _ order_hi order_lo (order_lo == 1) 0

            push 0
            eq
            push 0
            eq
            // Result found:     _ order_hi order_lo root (root == 1) (root_hi != 0)
            // Result not found: _ order_hi order_lo (order_lo == 1) (0 != 0)

            add
            push 0
            eq
            push 0
            eq
            // Result found:     _ order_hi order_lo root ((root == 1) || (root_hi != 0))
            // Result not found: _ order_hi order_lo ((order_lo == 1) || (0 != 0))

            assert error_id 141
            // Result found:     _ order_hi order_lo root
            // Result not found: VM crashed

            place 2
            pop 2

            return

        )
    }
}

impl Closure for PrimitiveRootOfUnity {
    fn rust_shadow(&self, stack: &mut Vec<BFieldElement>) {
        let order_lo: u32 = stack.pop().unwrap().try_into().unwrap();
        let order_hi: u32 = stack.pop().unwrap().try_into().unwrap();
        let order: u64 = order_lo as u64 + ((order_hi as u64) << 32);
        assert!(!order.is_zero(), "No root of order 0 exists");

        let root_of_unity = BFieldElement::primitive_root_of_unity(order).unwrap();

        stack.push(root_of_unity);
    }

    fn pseudorandom_initial_state(
        &self,
        seed: [u8; 32],
        bench_case: Option<BenchmarkCase>,
    ) -> Vec<BFieldElement> {
        let order = match bench_case {
            Some(BenchmarkCase::CommonCase) => 1_u64 << 10,
            Some(BenchmarkCase::WorstCase) => 1 << 32,
            None => 1 << StdRng::from_seed(seed).gen_range(1..=32),
        };

        let mut stack = empty_stack();
        stack.extend(order.encode().iter().rev());
        stack
    }
}

#[cfg(test)]
mod tests {
    use tasm_lib::test_helpers::test_assertion_failure;
    use triton_vm::prelude::*;

    use super::*;
    use crate::test_helpers::test_rust_equivalence_given_complete_state;
    use crate::traits::closure::ShadowedClosure;
    use crate::traits::rust_shadow::RustShadow;
    use crate::InitVmState;

    #[test]
    fn primitive_root_of_unity_pbt() {
        ShadowedClosure::new(PrimitiveRootOfUnity).test()
    }

    #[test]
    fn primitive_root_of_unity_unit_test() {
        for log2_order in 1..=32 {
            let order = 1u64 << log2_order;
            let mut init_stack = empty_stack();
            for elem in order.encode().iter().rev() {
                init_stack.push(*elem);
            }

            let expected = BFieldElement::primitive_root_of_unity(order).unwrap();
            let expected_final_stack = [empty_stack(), vec![expected]].concat();
            let _vm_output_state = test_rust_equivalence_given_complete_state(
                &ShadowedClosure::new(PrimitiveRootOfUnity),
                &init_stack,
                &[],
                &NonDeterminism::default(),
                &None,
                Some(&expected_final_stack),
            );
        }
    }

    #[test]
    fn primitive_root_negative_test() {
        let small_non_powers_of_two = (0_u64..100).filter(|x| !x.is_power_of_two());
        let larger_non_powers_of_two = (1_u64..50).map(|x| (1 << 32) - x);
        let too_large_powers_of_two = (33..64).map(|x| 1_u64 << x);

        for order in small_non_powers_of_two
            .chain(larger_non_powers_of_two)
            .chain(too_large_powers_of_two)
        {
            dbg!(order);
            let mut init_stack = empty_stack();
            init_stack.extend(order.encode().iter().rev());

            test_assertion_failure(
                &ShadowedClosure::new(PrimitiveRootOfUnity),
                InitVmState::with_stack(init_stack),
                &[140, 141],
            );
        }
    }
}

#[cfg(test)]
mod benches {
    use super::*;
    use crate::traits::closure::ShadowedClosure;
    use crate::traits::rust_shadow::RustShadow;

    #[test]
    fn bfe_primitive_root_of_unity_bench() {
        ShadowedClosure::new(PrimitiveRootOfUnity).bench()
    }
}
