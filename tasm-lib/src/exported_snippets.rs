use const_format::formatcp;
use triton_vm::proof_item::ProofItemVariant;
use triton_vm::table::challenges::Challenges;
use triton_vm::table::master_table::num_quotients;
use triton_vm::table::NUM_QUOTIENT_SEGMENTS;

use crate::arithmetic::u128::add_u128::AddU128;
use crate::arithmetic::u128::safe_mul_u128::SafeMulU128;
use crate::arithmetic::u128::shift_left_static_u128::ShiftLeftStaticU128;
use crate::arithmetic::u128::shift_left_u128::ShiftLeftU128;
use crate::arithmetic::u128::shift_right_static_u128::ShiftRightStaticU128;
use crate::arithmetic::u128::shift_right_u128::ShiftRightU128;
use crate::arithmetic::u128::sub_u128::SubU128;
use crate::arithmetic::u32::isodd::Isodd;
use crate::arithmetic::u32::isu32::Isu32;
use crate::arithmetic::u32::leadingzeros::Leadingzeros;
use crate::arithmetic::u32::or::Or;
use crate::arithmetic::u32::overflowingadd::Overflowingadd;
use crate::arithmetic::u32::safeadd::Safeadd;
use crate::arithmetic::u32::safemul::Safemul;
use crate::arithmetic::u32::safepow::Safepow;
use crate::arithmetic::u32::safesub::Safesub;
use crate::arithmetic::u32::shiftleft::Shiftleft;
use crate::arithmetic::u32::shiftright::Shiftright;
use crate::arithmetic::u64::add_u64::AddU64;
use crate::arithmetic::u64::and_u64::AndU64;
use crate::arithmetic::u64::decr_u64::DecrU64;
use crate::arithmetic::u64::div2_u64::Div2U64;
use crate::arithmetic::u64::div_mod_u64::DivModU64;
use crate::arithmetic::u64::double_pow2_u64::DoublePow2U64;
use crate::arithmetic::u64::eq_u64::EqU64;
use crate::arithmetic::u64::incr_u64::IncrU64;
use crate::arithmetic::u64::index_of_last_nonzero_bit::IndexOfLastNonZeroBitU64;
use crate::arithmetic::u64::leading_zeros_u64::LeadingZerosU64;
use crate::arithmetic::u64::log_2_floor_u64::Log2FloorU64;
use crate::arithmetic::u64::lt_u64::LtStandardU64;
use crate::arithmetic::u64::lt_u64::LtU64;
use crate::arithmetic::u64::mul_two_u64s_to_u128_u64::MulTwoU64sToU128;
use crate::arithmetic::u64::or_u64::OrU64;
use crate::arithmetic::u64::overflowing_sub_u64::OverflowingSub;
use crate::arithmetic::u64::popcount_u64::PopCountU64;
use crate::arithmetic::u64::pow2_u64::Pow2U64;
use crate::arithmetic::u64::safe_mul_u64::SafeMulU64;
use crate::arithmetic::u64::shift_left_u64::ShiftLeftU64;
use crate::arithmetic::u64::shift_right_u64::ShiftRightU64;
use crate::arithmetic::u64::sub_u64::SubU64;
use crate::arithmetic::u64::wrapping_mul_u64::WrappingMulU64;
use crate::arithmetic::u64::wrapping_sub_u64::WrappingSub;
use crate::arithmetic::u64::xor_u64::XorU64;
use crate::arithmetic::xfe::cube::Cube;
use crate::arithmetic::xfe::square::Square;
use crate::arithmetic::xfe::to_the_fourth::ToTheFourth;
use crate::array::horner_evaluation::HornerEvaluation;
use crate::array::inner_product_of_xfes::InnerProductOfXfes;
use crate::data_type::DataType;
use crate::hashing::algebraic_hasher;
use crate::hashing::eq_digest::EqDigest;
use crate::hashing::merkle_verify::MerkleVerify;
use crate::hashing::reverse_digest::ReverseDigest;
use crate::hashing::sponge_hasher;
use crate::hashing::swap_digest::SwapDigest;
use crate::io::read_input::ReadInput;
use crate::io::write_to_stdout::WriteToStdout;
use crate::io::InputSource;
use crate::list::contiguous_list;
use crate::list::get::Get;
use crate::list::length::Length;
use crate::list::new::New;
use crate::list::pop::Pop;
use crate::list::push::Push;
use crate::list::range::Range;
use crate::list::set::Set;
use crate::list::set_length::SetLength;
use crate::memory::dyn_malloc::DynMalloc;
use crate::memory::memcpy::MemCpy;
use crate::mmr::bag_peaks::BagPeaks;
use crate::mmr::calculate_new_peaks_from_append::CalculateNewPeaksFromAppend;
use crate::mmr::calculate_new_peaks_from_leaf_mutation::MmrCalculateNewPeaksFromLeafMutationMtIndices;
use crate::mmr::leaf_index_to_mt_index_and_peak_index::MmrLeafIndexToMtIndexAndPeakIndex;
use crate::mmr::verify_from_memory::MmrVerifyFromMemory;
use crate::mmr::verify_from_secret_in::MmrVerifyLeafMembershipFromSecretIn;
use crate::neptune::mutator_set::commit::Commit;
use crate::neptune::mutator_set::get_swbf_indices::GetSwbfIndices;
use crate::other_snippets::bfe_add::BfeAdd;
use crate::recufier::challenges::new_empty_input_and_output::NewEmptyInputAndOutput;
use crate::recufier::master_ext_table::air_constraint_evaluation::AirConstraintEvaluation;
use crate::recufier::own_program_digest::OwnProgramDigest;
use crate::recufier::proof_stream::dequeue_next_as::DequeueNextAs;
use crate::recufier::read_and_verify_own_program_digest_from_std_in::ReadAndVerifyOwnProgramDigestFromStdIn;
use crate::traits::basic_snippet::BasicSnippet;

const NUM_CONSTRAINTS_TVM: usize = num_quotients();
const WEIGHTS_QUOTIENTS_INNER_PRODUCT_ENTRYPOINT: &str =
    formatcp!("tasm_array_inner_product_of_{}_xfes", NUM_CONSTRAINTS_TVM);
const HORNER_EVALUATION_FOR_SUM_OF_EVALUATED_OUT_OF_DOMAIN_QUOTIENT_SEGMENTS_ENTRYPOINT: &str = formatcp!(
    "tasm_array_horner_evaluation_with_{}_coefficients",
    NUM_QUOTIENT_SEGMENTS
);

pub fn name_to_snippet(fn_name: &str) -> Box<dyn BasicSnippet> {
    match fn_name {
        // XFieldElement
        "tasm_arithmetic_xfe_square" => Box::new(Square),
        "tasm_arithmetic_xfe_cube" => Box::new(Cube),
        "tasm_arithmetic_xfe_to_the_fourth" => Box::new(ToTheFourth),

        // u32
        "tasm_arithmetic_u32_isodd" => Box::new(Isodd),
        "tasm_arithmetic_u32_isu32" => Box::new(Isu32),
        "tasm_arithmetic_u32_safeadd" => Box::new(Safeadd),
        "tasm_arithmetic_u32_safesub" => Box::new(Safesub),
        "tasm_arithmetic_u32_safemul" => Box::new(Safemul),
        "tasm_arithmetic_u32_shiftright" => Box::new(Shiftright),
        "tasm_arithmetic_u32_shiftleft" => Box::new(Shiftleft),
        "tasm_arithmetic_u32_or" => Box::new(Or),
        "tasm_arithmetic_u32_leadingzeros" => Box::new(Leadingzeros),
        "tasm_arithmetic_u32_safepow" => Box::new(Safepow),
        "tasm_arithmetic_u32_overflowingadd" => Box::new(Overflowingadd),

        // u64
        "tasm_arithmetic_u64_add" => Box::new(AddU64),
        "tasm_arithmetic_u64_and" => Box::new(AndU64),
        "tasm_arithmetic_u64_xor" => Box::new(XorU64),
        "tasm_arithmetic_u64_or_u64" => Box::new(OrU64),
        "tasm_arithmetic_u64_decr" => Box::new(DecrU64),
        "tasm_arithmetic_u64_div2" => Box::new(Div2U64),
        "tasm_arithmetic_u64_div_mod" => Box::new(DivModU64),
        "tasm_arithmetic_u64_eq" => Box::new(EqU64),
        "tasm_arithmetic_u64_incr" => Box::new(IncrU64),
        "tasm_arithmetic_u64_log_2_floor" => Box::new(Log2FloorU64),
        "tasm_arithmetic_u64_lt" => Box::new(LtU64),
        "tasm_arithmetic_u64_lt_standard" => Box::new(LtStandardU64),
        "tasm_arithmetic_u64_pow2" => Box::new(Pow2U64),
        "tasm_arithmetic_u64_sub" => Box::new(SubU64),
        "tasm_arithmetic_u64_leading_zeros" => Box::new(LeadingZerosU64),
        "tasm_arithmetic_u64_index_of_last_nonzero_bit" => Box::new(IndexOfLastNonZeroBitU64),
        "tasm_arithmetic_u64_pow2_double" => Box::new(DoublePow2U64),
        "tasm_arithmetic_u64_wrapping_mul" => Box::new(WrappingMulU64),
        "tasm_arithmetic_u64_safe_mul" => Box::new(SafeMulU64),
        "tasm_arithmetic_u64_popcount" => Box::new(PopCountU64),
        "tasm_arithmetic_u64_shift_right" => Box::new(ShiftRightU64),
        "tasm_arithmetic_u64_shift_left" => Box::new(ShiftLeftU64),
        "tasm_arithmetic_u64_mul_two_u64s_to_u128_u64" => Box::new(MulTwoU64sToU128),
        "tasm_arithmetic_u64_wrapping_sub" => Box::new(WrappingSub),
        "tasm_arithmetic_u64_overflowing_sub" => Box::new(OverflowingSub),

        // u128
        "tasm_arithmetic_u128_add" => Box::new(AddU128),
        "tasm_arithmetic_u128_shift_left" => Box::new(ShiftLeftU128),
        "tasm_arithmetic_u128_shift_right" => Box::new(ShiftRightU128),
        "tasm_arithmetic_u128_sub" => Box::new(SubU128),
        "tasm_arithmetic_u128_safe_mul" => Box::new(SafeMulU128),

        "tasm_arithmetic_u128_shift_left_static_1" => Box::new(ShiftLeftStaticU128::<1>),
        "tasm_arithmetic_u128_shift_left_static_2" => Box::new(ShiftLeftStaticU128::<2>),
        "tasm_arithmetic_u128_shift_left_static_3" => Box::new(ShiftLeftStaticU128::<3>),
        "tasm_arithmetic_u128_shift_left_static_4" => Box::new(ShiftLeftStaticU128::<4>),
        "tasm_arithmetic_u128_shift_left_static_5" => Box::new(ShiftLeftStaticU128::<5>),
        "tasm_arithmetic_u128_shift_left_static_6" => Box::new(ShiftLeftStaticU128::<6>),
        "tasm_arithmetic_u128_shift_left_static_7" => Box::new(ShiftLeftStaticU128::<7>),
        "tasm_arithmetic_u128_shift_left_static_8" => Box::new(ShiftLeftStaticU128::<8>),
        "tasm_arithmetic_u128_shift_left_static_9" => Box::new(ShiftLeftStaticU128::<9>),
        "tasm_arithmetic_u128_shift_left_static_10" => Box::new(ShiftLeftStaticU128::<10>),
        "tasm_arithmetic_u128_shift_left_static_11" => Box::new(ShiftLeftStaticU128::<11>),
        "tasm_arithmetic_u128_shift_left_static_12" => Box::new(ShiftLeftStaticU128::<12>),
        "tasm_arithmetic_u128_shift_left_static_13" => Box::new(ShiftLeftStaticU128::<13>),
        "tasm_arithmetic_u128_shift_left_static_14" => Box::new(ShiftLeftStaticU128::<14>),
        "tasm_arithmetic_u128_shift_left_static_15" => Box::new(ShiftLeftStaticU128::<15>),
        "tasm_arithmetic_u128_shift_left_static_16" => Box::new(ShiftLeftStaticU128::<16>),
        "tasm_arithmetic_u128_shift_left_static_17" => Box::new(ShiftLeftStaticU128::<17>),
        "tasm_arithmetic_u128_shift_left_static_18" => Box::new(ShiftLeftStaticU128::<18>),
        "tasm_arithmetic_u128_shift_left_static_19" => Box::new(ShiftLeftStaticU128::<19>),
        "tasm_arithmetic_u128_shift_left_static_20" => Box::new(ShiftLeftStaticU128::<20>),
        "tasm_arithmetic_u128_shift_left_static_21" => Box::new(ShiftLeftStaticU128::<21>),
        "tasm_arithmetic_u128_shift_left_static_22" => Box::new(ShiftLeftStaticU128::<22>),
        "tasm_arithmetic_u128_shift_left_static_23" => Box::new(ShiftLeftStaticU128::<23>),
        "tasm_arithmetic_u128_shift_left_static_24" => Box::new(ShiftLeftStaticU128::<24>),
        "tasm_arithmetic_u128_shift_left_static_25" => Box::new(ShiftLeftStaticU128::<25>),
        "tasm_arithmetic_u128_shift_left_static_26" => Box::new(ShiftLeftStaticU128::<26>),
        "tasm_arithmetic_u128_shift_left_static_27" => Box::new(ShiftLeftStaticU128::<27>),
        "tasm_arithmetic_u128_shift_left_static_28" => Box::new(ShiftLeftStaticU128::<28>),
        "tasm_arithmetic_u128_shift_left_static_29" => Box::new(ShiftLeftStaticU128::<29>),
        "tasm_arithmetic_u128_shift_left_static_30" => Box::new(ShiftLeftStaticU128::<30>),
        "tasm_arithmetic_u128_shift_left_static_31" => Box::new(ShiftLeftStaticU128::<31>),
        "tasm_arithmetic_u128_shift_left_static_32" => Box::new(ShiftLeftStaticU128::<32>),

        "tasm_arithmetic_u128_shift_right_static_1" => Box::new(ShiftRightStaticU128::<1>),
        "tasm_arithmetic_u128_shift_right_static_2" => Box::new(ShiftRightStaticU128::<2>),
        "tasm_arithmetic_u128_shift_right_static_3" => Box::new(ShiftRightStaticU128::<3>),
        "tasm_arithmetic_u128_shift_right_static_4" => Box::new(ShiftRightStaticU128::<4>),
        "tasm_arithmetic_u128_shift_right_static_5" => Box::new(ShiftRightStaticU128::<5>),
        "tasm_arithmetic_u128_shift_right_static_6" => Box::new(ShiftRightStaticU128::<6>),
        "tasm_arithmetic_u128_shift_right_static_7" => Box::new(ShiftRightStaticU128::<7>),
        "tasm_arithmetic_u128_shift_right_static_8" => Box::new(ShiftRightStaticU128::<8>),
        "tasm_arithmetic_u128_shift_right_static_9" => Box::new(ShiftRightStaticU128::<9>),
        "tasm_arithmetic_u128_shift_right_static_10" => Box::new(ShiftRightStaticU128::<10>),
        "tasm_arithmetic_u128_shift_right_static_11" => Box::new(ShiftRightStaticU128::<11>),
        "tasm_arithmetic_u128_shift_right_static_12" => Box::new(ShiftRightStaticU128::<12>),
        "tasm_arithmetic_u128_shift_right_static_13" => Box::new(ShiftRightStaticU128::<13>),
        "tasm_arithmetic_u128_shift_right_static_14" => Box::new(ShiftRightStaticU128::<14>),
        "tasm_arithmetic_u128_shift_right_static_15" => Box::new(ShiftRightStaticU128::<15>),
        "tasm_arithmetic_u128_shift_right_static_16" => Box::new(ShiftRightStaticU128::<16>),
        "tasm_arithmetic_u128_shift_right_static_17" => Box::new(ShiftRightStaticU128::<17>),
        "tasm_arithmetic_u128_shift_right_static_18" => Box::new(ShiftRightStaticU128::<18>),
        "tasm_arithmetic_u128_shift_right_static_19" => Box::new(ShiftRightStaticU128::<19>),
        "tasm_arithmetic_u128_shift_right_static_20" => Box::new(ShiftRightStaticU128::<20>),
        "tasm_arithmetic_u128_shift_right_static_21" => Box::new(ShiftRightStaticU128::<21>),
        "tasm_arithmetic_u128_shift_right_static_22" => Box::new(ShiftRightStaticU128::<22>),
        "tasm_arithmetic_u128_shift_right_static_23" => Box::new(ShiftRightStaticU128::<23>),
        "tasm_arithmetic_u128_shift_right_static_24" => Box::new(ShiftRightStaticU128::<24>),
        "tasm_arithmetic_u128_shift_right_static_25" => Box::new(ShiftRightStaticU128::<25>),
        "tasm_arithmetic_u128_shift_right_static_26" => Box::new(ShiftRightStaticU128::<26>),
        "tasm_arithmetic_u128_shift_right_static_27" => Box::new(ShiftRightStaticU128::<27>),
        "tasm_arithmetic_u128_shift_right_static_28" => Box::new(ShiftRightStaticU128::<28>),
        "tasm_arithmetic_u128_shift_right_static_29" => Box::new(ShiftRightStaticU128::<29>),
        "tasm_arithmetic_u128_shift_right_static_30" => Box::new(ShiftRightStaticU128::<30>),
        "tasm_arithmetic_u128_shift_right_static_31" => Box::new(ShiftRightStaticU128::<31>),
        "tasm_arithmetic_u128_shift_right_static_32" => Box::new(ShiftRightStaticU128::<32>),

        // Hashing
        "tasm_hashing_eq_digest" => Box::new(EqDigest),
        "tasm_hashing_swap_digest" => Box::new(SwapDigest),
        "tasm_hashing_reverse_digest" => Box::new(ReverseDigest),
        "tasm_hashing_merkle_verify" => Box::new(MerkleVerify),

        // Hashing -> algebraic hasher trait
        "tasm_hashing_algebraic_hasher_hash_varlen" => Box::new(algebraic_hasher::hash_varlen::HashVarlen),
        "tasm_hashing_algebraic_hasher_sample_indices" => Box::new(algebraic_hasher::sample_indices::SampleIndices),
        "tasm_hashing_algebraic_hasher_sample_scalars" => Box::new(algebraic_hasher::sample_scalars::SampleScalars),

        // Hashing -> Sponge hasher trait
        "tasm_hashing_sponge_hasher_init" => Box::new(sponge_hasher::init::Init),
        "tasm_hashing_sponge_hasher_absorb" => Box::new(sponge_hasher::absorb::Absorb),
        "tasm_hashing_sponge_hasher_squeeze" => Box::new(sponge_hasher::squeeze::Squeeze),
        "tasm_hashing_sponge_hasher_pad_and_absorb_all" => Box::new(sponge_hasher::pad_and_absorb_all::PadAndAbsorbAll),

        // io
        "tasm_io_read_secin___bool" => Box::new(ReadInput {
            data_type: DataType::Bool,
            input_source: InputSource::SecretIn,
        }),
        "tasm_io_read_secin___u32" => Box::new(ReadInput {
            data_type: DataType::U32,
            input_source: InputSource::SecretIn,
        }),
        "tasm_io_read_secin___u64" => Box::new(ReadInput {
            data_type: DataType::U64,
            input_source: InputSource::SecretIn,
        }),
        "tasm_io_read_secin___u128" => Box::new(ReadInput {
            data_type: DataType::U128,
            input_source: InputSource::SecretIn,
        }),
        "tasm_io_read_secin___bfe" => Box::new(ReadInput {
            data_type: DataType::Bfe,
            input_source: InputSource::SecretIn,
        }),
        "tasm_io_read_secin___xfe" => Box::new(ReadInput {
            data_type: DataType::Xfe,
            input_source: InputSource::SecretIn,
        }),
        "tasm_io_read_secin___digest" => Box::new(ReadInput {
            data_type: DataType::Digest,
            input_source: InputSource::SecretIn,
        }),

        "tasm_io_read_stdin___bool" => Box::new(ReadInput {
            data_type: DataType::Bool,
            input_source: InputSource::StdIn,
        }),
        "tasm_io_read_stdin___u32" => Box::new(ReadInput {
            data_type: DataType::U32,
            input_source: InputSource::StdIn,
        }),
        "tasm_io_read_stdin___u64" => Box::new(ReadInput {
            data_type: DataType::U64,
            input_source: InputSource::StdIn,
        }),
        "tasm_io_read_stdin___u128" => Box::new(ReadInput {
            data_type: DataType::U128,
            input_source: InputSource::StdIn,
        }),
        "tasm_io_read_stdin___bfe" => Box::new(ReadInput {
            data_type: DataType::Bfe,
            input_source: InputSource::StdIn,
        }),
        "tasm_io_read_stdin___xfe" => Box::new(ReadInput {
            data_type: DataType::Xfe,
            input_source: InputSource::StdIn,
        }),
        "tasm_io_read_stdin___digest" => Box::new(ReadInput {
            data_type: DataType::Digest,
            input_source: InputSource::StdIn,
        }),

        "tasm_io_write_to_stdout___bool" => Box::new(WriteToStdout { data_type: DataType::Bool }),
        "tasm_io_write_to_stdout___u32" => Box::new(WriteToStdout { data_type: DataType::U32 }),
        "tasm_io_write_to_stdout___u64" => Box::new(WriteToStdout { data_type: DataType::U64 }),
        "tasm_io_write_to_stdout___u128" => Box::new(WriteToStdout { data_type: DataType::U128 }),
        "tasm_io_write_to_stdout___bfe" => Box::new(WriteToStdout { data_type: DataType::Bfe }),
        "tasm_io_write_to_stdout___xfe" => Box::new(WriteToStdout { data_type: DataType::Xfe }),
        "tasm_io_write_to_stdout___digest" => Box::new(WriteToStdout { data_type: DataType::Digest }),

        // lists
        "tasm_list_get_element___bool" => Box::new(Get::new(DataType::Bool)),
        "tasm_list_get_element___u32" => Box::new(Get::new(DataType::U32)),
        "tasm_list_get_element___u64" => Box::new(Get::new(DataType::U64)),
        "tasm_list_get_element___bfe" => Box::new(Get::new(DataType::Bfe)),
        "tasm_list_get_element___xfe" => Box::new(Get::new(DataType::Xfe)),
        "tasm_list_get_element___digest" => Box::new(Get::new(DataType::Digest)),

        "tasm_list_pop___bool" => Box::new(Pop::new(DataType::Bool)),
        "tasm_list_pop___u32" => Box::new(Pop::new(DataType::U32)),
        "tasm_list_pop___u64" => Box::new(Pop::new(DataType::U64)),
        "tasm_list_pop___bfe" => Box::new(Pop::new(DataType::Bfe)),
        "tasm_list_pop___xfe" => Box::new(Pop::new(DataType::Xfe)),
        "tasm_list_pop___digest" => Box::new(Pop::new(DataType::Digest)),

        "tasm_list_push___bool" => Box::new(Push::new(DataType::Bool)),
        "tasm_list_push___u32" => Box::new(Push::new(DataType::U32)),
        "tasm_list_push___u64" => Box::new(Push::new(DataType::U64)),
        "tasm_list_push___bfe" => Box::new(Push::new(DataType::Bfe)),
        "tasm_list_push___xfe" => Box::new(Push::new(DataType::Xfe)),
        "tasm_list_push___digest" => Box::new(Push::new(DataType::Digest)),

        "tasm_list_set_element___bool" => Box::new(Set::new(DataType::Bool)),
        "tasm_list_set_element___u32" => Box::new(Set::new(DataType::U32)),
        "tasm_list_set_element___u64" => Box::new(Set::new(DataType::U64)),
        "tasm_list_set_element___bfe" => Box::new(Set::new(DataType::Bfe)),
        "tasm_list_set_element___xfe" => Box::new(Set::new(DataType::Xfe)),
        "tasm_list_set_element___digest" => Box::new(Set::new(DataType::Digest)),

        "tasm_list_new___bool" => Box::new(New::new(DataType::Bool)),
        "tasm_list_new___u32" => Box::new(New::new(DataType::U32)),
        "tasm_list_new___u64" => Box::new(New::new(DataType::U64)),
        "tasm_list_new___bfe" => Box::new(New::new(DataType::Bfe)),
        "tasm_list_new___xfe" => Box::new(New::new(DataType::Xfe)),
        "tasm_list_new___digest" => Box::new(New::new(DataType::Digest)),

        "tasm_list_length___bool" => Box::new(Length::new(DataType::Bool)),
        "tasm_list_length___u32" => Box::new(Length::new(DataType::U32)),
        "tasm_list_length___u64" => Box::new(Length::new(DataType::U64)),
        "tasm_list_length___bfe" => Box::new(Length::new(DataType::Bfe)),
        "tasm_list_length___xfe" => Box::new(Length::new(DataType::Xfe)),
        "tasm_list_length___digest" => Box::new(Length::new(DataType::Digest)),

        "tasm_list_set_length___bool" => Box::new(SetLength::new(DataType::Bool)),
        "tasm_list_set_length___u32" => Box::new(SetLength::new(DataType::U32)),
        "tasm_list_set_length___u64" => Box::new(SetLength::new(DataType::U64)),
        "tasm_list_set_length___bfe" => Box::new(SetLength::new(DataType::Bfe)),
        "tasm_list_set_length___xfe" => Box::new(SetLength::new(DataType::Xfe)),
        "tasm_list_set_length___digest" => Box::new(SetLength::new(DataType::Digest)),

        "tasm_list_multiset_equality" => Box::new(crate::list::multiset_equality::MultisetEquality),
        "tasm_list_range" => Box::new(Range),

        // Contiguous lists
        "tasm_list_contiguous_list_get_length" => Box::new(contiguous_list::get_length::GetLength),
        "tasm_list_contiguous_list_get_pointer_list" => Box::new(contiguous_list::get_pointer_list::GetPointerList),

        // MMR
        "tasm_mmr_calculate_new_peaks_from_append" => Box::new(CalculateNewPeaksFromAppend),
        "tasm_mmr_calculate_new_peaks_from_leaf_mutation" => {
            Box::new(MmrCalculateNewPeaksFromLeafMutationMtIndices)
        }
        "tasm_mmr_leaf_index_to_mt_index_and_peak_index" => Box::new(MmrLeafIndexToMtIndexAndPeakIndex),
        "tasm_mmr_verify_from_secret_in" => Box::new(MmrVerifyLeafMembershipFromSecretIn),
        "tasm_mmr_bag_peaks" => Box::new(BagPeaks),
        "tasm_mmr_verify_from_memory" => Box::new(MmrVerifyFromMemory),

        // other
        "tasm_other_bfe_add" => Box::new(BfeAdd),

        // recufy
        "tasm_recufier_proof_stream_dequeue_next_as_merkleroot" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::MerkleRoot })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_outofdomainbaserow" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::OutOfDomainBaseRow })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_outofdomainextrow" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::OutOfDomainExtRow })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_outofdomainquotientsegments" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::OutOfDomainQuotientSegments })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_authenticationstructure" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::AuthenticationStructure })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_masterbasetablerows" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::MasterBaseTableRows })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_masterexttablerows" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::MasterExtTableRows })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_log2paddedheight" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::Log2PaddedHeight })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_quotientsegmentselements" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::QuotientSegmentsElements })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_fricodeword" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::FriCodeword })
        }
        "tasm_recufier_proof_stream_dequeue_next_as_friresponse" => {
            Box::new(DequeueNextAs { proof_item: ProofItemVariant::FriResponse })
        }
        "tasm_recufier_read_and_verify_own_program_digest_from_std_in" => {
            Box::new(ReadAndVerifyOwnProgramDigestFromStdIn)
        }
        "tasm_recufier_challenges_new_empty_input_and_output_59_4" => {
            let num_challenges_to_sample = Challenges::SAMPLE_COUNT;
            let num_challenges_to_compute = Challenges::COUNT - num_challenges_to_sample;
            assert_eq!(59, num_challenges_to_sample);
            assert_eq!(4, num_challenges_to_compute);
            let challenge_snippet
                = NewEmptyInputAndOutput::new(num_challenges_to_sample, num_challenges_to_compute, NewEmptyInputAndOutput::conventional_challenges_pointer());
            Box::new(challenge_snippet)
        }
        "tasm_recufier_master_ext_table_air_constraint_evaluation" => {
            Box::new(AirConstraintEvaluation::with_conventional_memory_layout())
        }

        "tasm_array_inner_product_of_4_xfes" => {
            Box::new(InnerProductOfXfes::new(4))
        }
        WEIGHTS_QUOTIENTS_INNER_PRODUCT_ENTRYPOINT => {
            Box::new(InnerProductOfXfes::new(NUM_CONSTRAINTS_TVM))
        }
        HORNER_EVALUATION_FOR_SUM_OF_EVALUATED_OUT_OF_DOMAIN_QUOTIENT_SEGMENTS_ENTRYPOINT => {
            Box::new(HornerEvaluation::new(NUM_QUOTIENT_SEGMENTS))
        }
        "tasm_recufier_own_program_digest" => {
            Box::new(OwnProgramDigest)
        }

        // memory
        "tasm_memory_dyn_malloc" => Box::new(DynMalloc),
        "tasm_memory_memcpy" => Box::new(MemCpy),

        // FRI
        #[cfg(not(test))]
        "tasm_recufier_fri_verify" => Box::new(crate::recufier::fri_verify::FriSnippet {}),

        // structure

        // mutator sets
        "tasm_neptune_mutator_set_commit" => Box::new(Commit),
        "tasm_neptune_mutator_get_swbf_indices_1048576_45" => Box::new(GetSwbfIndices { window_size: 1048576, num_trials: 45 }),

        _ => panic!("Could not find \"{fn_name}\" in the function `exported_snippets`. Did you include it there?"),
    }
}
