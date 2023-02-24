use std::fs::{create_dir_all, File};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::to_writer_pretty;
use triton_vm::table::master_table::MasterBaseTable;

use crate::snippet::{simulate_snippet, Snippet};

#[derive(Debug, Serialize, Deserialize)]
pub struct SnippetBenchmark {
    name: String,
    processor_table_height: usize,
    hash_table_height: usize,
    u32_table_height: usize,
    case: SnippetBenchmarkCase,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum SnippetBenchmarkCase {
    CommonCase,
    WorstCase,
}

#[allow(dead_code)]
pub fn benchmark_snippet<T: Snippet + Clone>(snippet: T) -> Vec<SnippetBenchmark> {
    let mut benchmarks = Vec::with_capacity(2);

    for (case, execution_state) in [
        (
            SnippetBenchmarkCase::CommonCase,
            snippet.common_case_input_state(),
        ),
        (
            SnippetBenchmarkCase::WorstCase,
            snippet.worst_case_input_state(),
        ),
    ] {
        let (aet, inflated_clock_cycles) = simulate_snippet::<T>(snippet.clone(), execution_state);
        let benchmark = SnippetBenchmark {
            name: snippet.entrypoint(),
            processor_table_height: aet.processor_trace.nrows() - inflated_clock_cycles,
            hash_table_height: aet.hash_trace.nrows(),
            u32_table_height: MasterBaseTable::u32_table_length(&aet),
            case,
        };
        benchmarks.push(benchmark);
    }

    benchmarks
}

#[allow(dead_code)]
pub fn write_benchmarks<T: Snippet>(benchmarks: Vec<SnippetBenchmark>, snippet: T) {
    let mut path = PathBuf::new();
    path.push("benchmarks");
    create_dir_all(&path).expect("benchmarks directory should exist");

    path.push(Path::new(&snippet.entrypoint()).with_extension("json"));
    let output = File::create(&path).expect("open file for writing");
    to_writer_pretty(output, &benchmarks).expect("write json to file");
}

#[allow(dead_code)]
pub fn bench_and_write<T: Snippet + Clone>(snippet: T) {
    write_benchmarks::<T>(benchmark_snippet::<T>(snippet.clone()), snippet);
}
