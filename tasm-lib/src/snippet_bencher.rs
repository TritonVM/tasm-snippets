use std::fs::create_dir_all;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

use serde::Deserialize;
use serde::Serialize;
use serde_json::to_writer_pretty;
use triton_vm::aet::AlgebraicExecutionTrace;
use triton_vm::table::master_table::TableId;

use crate::traits::deprecated_snippet::DeprecatedSnippet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub clock_cycle_count: usize,
    pub hash_table_height: usize,
    pub u32_table_height: usize,
    pub op_stack_table_height: usize,
    pub ram_table_height: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedBenchmarkResult {
    pub name: String,
    pub benchmark_result: BenchmarkResult,
    pub case: BenchmarkCase,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum BenchmarkCase {
    CommonCase,
    WorstCase,
}

impl BenchmarkResult {
    pub fn new(aet: &AlgebraicExecutionTrace) -> Self {
        BenchmarkResult {
            clock_cycle_count: aet.height_of_table(TableId::Processor),
            hash_table_height: aet.height_of_table(TableId::Hash),
            u32_table_height: aet.height_of_table(TableId::U32),
            op_stack_table_height: aet.height_of_table(TableId::OpStack),
            ram_table_height: aet.height_of_table(TableId::Ram),
        }
    }
}

#[allow(dead_code)]
pub fn benchmark_snippet_deprecated<T: DeprecatedSnippet>(snippet: T) -> Vec<NamedBenchmarkResult> {
    let mut benchmarks = Vec::with_capacity(2);

    for (case, mut execution_state) in [
        (BenchmarkCase::CommonCase, snippet.common_case_input_state()),
        (BenchmarkCase::WorstCase, snippet.worst_case_input_state()),
    ] {
        let benchmark_result = snippet
            .link_and_run_tasm_from_state_for_bench(&mut execution_state)
            .unwrap();
        let benchmark = NamedBenchmarkResult {
            name: snippet.entrypoint_name(),
            benchmark_result,
            case,
        };
        benchmarks.push(benchmark);
    }

    benchmarks
}

#[allow(dead_code)]
pub fn write_benchmarks(benchmarks: Vec<NamedBenchmarkResult>) {
    let mut path = PathBuf::new();
    path.push("benchmarks");
    create_dir_all(&path).expect("benchmarks directory should exist");

    let function_name = &benchmarks[0].name;
    for fnname in benchmarks.iter().map(|x| &x.name) {
        assert_eq!(
            function_name, fnname,
            "all fn names must agree for benchmark writing to disk"
        );
    }

    path.push(Path::new(&function_name).with_extension("json"));
    let output = File::create(&path).expect("open file for writing");
    to_writer_pretty(output, &benchmarks).expect("write json to file");
}

#[allow(dead_code)]
pub fn bench_and_write<T: DeprecatedSnippet>(snippet: T) {
    write_benchmarks(benchmark_snippet_deprecated(snippet));
}
