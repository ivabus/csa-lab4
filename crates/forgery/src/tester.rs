use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Mutex,
    time::Instant,
};

use color_eyre::eyre::{self, Context, ContextCompat};
use emulator::{IO, Processor, cpu};
use serde::{Deserialize, Serialize};

use crate::grint;

static TRACE_BUF: Mutex<String> = Mutex::new(String::new());

fn capture_trace(s: &str) {
    TRACE_BUF.lock().unwrap().push_str(s);
}

#[derive(Serialize, Deserialize)]
struct TestFile {
    #[serde(rename = "test")]
    tests: Vec<Test>,
    source_file: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct Test {
    name: Option<String>,
    input: Option<String>,
    asserted_output: Option<String>,
    max_steps: Option<u64>,
    generator: Option<PathBuf>,
    iterations: Option<u64>,
    #[serde(default)]
    trace_level: u8,
    trace_ranges: Option<String>,
    trace_assertion: Option<PathBuf>,
}

struct TraceRange {
    start: Option<isize>,
    end: Option<isize>,
    inclusive: bool,
}

fn parse_trace_ranges(s: &str) -> eyre::Result<Vec<TraceRange>> {
    s.split('|')
        .filter(|s| !s.is_empty())
        .map(|part| {
            if part == ".." {
                return Ok(TraceRange {
                    start: None,
                    end: None,
                    inclusive: false,
                });
            }
            if let Some(pos) = part.find("..=") {
                let left = &part[..pos];
                let right = &part[pos + 3..];
                Ok(TraceRange {
                    start: if left.is_empty() {
                        None
                    } else {
                        Some(
                            left.parse().wrap_err_with(|| {
                                format!("Invalid trace range start in '{part}'")
                            })?,
                        )
                    },
                    end: if right.is_empty() {
                        None
                    } else {
                        Some(
                            right
                                .parse()
                                .wrap_err_with(|| format!("Invalid trace range end in '{part}'"))?,
                        )
                    },
                    inclusive: true,
                })
            } else if let Some(pos) = part.find("..") {
                let left = &part[..pos];
                let right = &part[pos + 2..];
                Ok(TraceRange {
                    start: if left.is_empty() {
                        None
                    } else {
                        Some(
                            left.parse().wrap_err_with(|| {
                                format!("Invalid trace range start in '{part}'")
                            })?,
                        )
                    },
                    end: if right.is_empty() {
                        None
                    } else {
                        Some(
                            right
                                .parse()
                                .wrap_err_with(|| format!("Invalid trace range end in '{part}'"))?,
                        )
                    },
                    inclusive: false,
                })
            } else {
                Err(eyre::eyre!("Invalid trace range: {}", part))
            }
        })
        .collect()
}

fn apply_trace_ranges(trace: &str, ranges: &[TraceRange]) -> String {
    let lines: Vec<&str> = trace.lines().collect();
    let total = lines.len();
    let mut selected: Vec<usize> = Vec::new();

    for range in ranges {
        let start = match range.start {
            Some(n) if n < 0 => {
                let from_end = total as isize + n;
                if from_end < 0 { 0 } else { from_end as usize }
            }
            Some(n) => n as usize,
            None => 0,
        };

        let end = match range.end {
            Some(n) if n < 0 => {
                let from_end = total as isize + n;
                if from_end < 0 { 0 } else { from_end as usize }
            }
            Some(n) => n as usize,
            None => total,
        };

        let end_idx = if range.inclusive {
            end.saturating_add(1)
        } else {
            end
        };

        for i in start..end_idx.min(total) {
            if !selected.contains(&i) {
                selected.push(i);
            }
        }
    }

    selected.sort();
    selected.dedup();

    let mut result = String::new();
    for (k, &i) in selected.iter().enumerate() {
        if k > 0 {
            result.push('\n');
        }
        result.push_str(lines[i]);
    }
    result
}

impl Test {
    fn run(&self, binary: &[u8], test_dir: &Path) -> eyre::Result<()> {
        // Configure trace target
        if self.trace_level > 0 {
            *cpu::TRACE_TARGET.write().unwrap() = capture_trace;
            *TRACE_BUF.lock().unwrap() = String::new();
        }

        for it in 0..self.iterations.unwrap_or(1) {
            let mut memory = BTreeMap::new();
            for (i, chunk) in binary.chunks(4).enumerate() {
                if chunk.len() == 4 {
                    let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    memory.insert(0x1000 + (i as u32 * 4), val);
                }
            }

            let start = Instant::now();
            let mut out = Vec::new();
            let has_output_assert = self.asserted_output.is_some() || self.generator.is_some();
            if has_output_assert {
                let (input, asserted_output) = match (&self.input, &self.asserted_output) {
                    (Some(input), Some(output)) => (input.as_bytes().to_vec(), output.clone()),
                    (None, Some(output)) => (vec![], output.clone()),
                    (None, None) if let Some(generator) = &self.generator => {
                        let child = std::process::Command::new(generator)
                            .stdout(Stdio::piped())
                            .spawn()?;
                        let g_out = child.wait_with_output()?.stdout;

                        #[derive(Deserialize)]
                        struct GeneratorResult {
                            input: String,
                            asserted_output: String,
                        }
                        let res: GeneratorResult = toml::from_slice(&g_out)
                            .context("Generator failed to provide valid toml")?;

                        (res.input.as_bytes().to_vec(), res.asserted_output)
                    }
                    _ => unreachable!(),
                };

                let output = Cursor::new(&mut out);
                let mut io_map = BTreeMap::new();
                io_map.insert(0x0000_0000, IO::I(Box::new(input.as_slice())));
                io_map.insert(0x0000_0004, IO::O(Box::new(output)));

                let mut cpu = Processor::new(memory, io_map);
                cpu.trace = self.trace_level;

                let mut limit = self.max_steps.unwrap_or(1_000_000_000);
                while cpu.step() && limit > 0 {
                    limit -= 1;
                }
                drop(cpu);
                let out = String::from_utf8(out)?;
                if out != asserted_output {
                    eyre::bail!(
                        "Assertion FAILED, left != right!\nleft (got): {}\nright (asserted): {}",
                        out.escape_debug(),
                        asserted_output.escape_debug()
                    )
                }
            } else {
                // Trace-only mode: no output assertion, run the program silently
                let io_map = BTreeMap::new();
                let mut cpu = Processor::new(memory, io_map);
                cpu.trace = self.trace_level;

                let mut limit = self.max_steps.unwrap_or(1_000_000_000);
                while cpu.step() && limit > 0 {
                    limit -= 1;
                }
            }
            if self.iterations.unwrap_or(1) != 1 {
                grint!(
                    "Success",
                    "Iteration {} of test {} completed in {:.5}s",
                    it,
                    self.name.as_ref().unwrap(),
                    start.elapsed().as_secs_f32()
                );
            }
        }

        // Trace assertion
        if self.trace_level > 0 {
            let full_trace = TRACE_BUF.lock().unwrap().clone();
            let filtered = if let Some(ranges_str) = &self.trace_ranges {
                let ranges = parse_trace_ranges(ranges_str)?;
                apply_trace_ranges(&full_trace, &ranges)
            } else {
                full_trace
            };

            if let Some(assertion_path) = &self.trace_assertion {
                let golden_path = test_dir.join(assertion_path);
                let golden = std::fs::read_to_string(&golden_path).wrap_err_with(|| {
                    format!("Failed to read trace assertion: {:?}", golden_path)
                })?;

                let normalize = |s: &str| -> String {
                    s.lines()
                        .map(|l| l.trim_end_matches(' '))
                        .collect::<Vec<_>>()
                        .join("\n")
                        .trim_end()
                        .to_string()
                };

                if normalize(&filtered) != normalize(&golden) {
                    eyre::bail!(
                        "Trace assertion FAILED!\n--- expected ({:?}) ---\n{}\n--- got ---\n{}",
                        golden_path,
                        golden,
                        filtered
                    );
                }
            }
        }

        Ok(())
    }
}

pub fn test(path: impl AsRef<Path>) -> eyre::Result<()> {
    let test_dir = path
        .as_ref()
        .parent()
        .context("Failed to get test directory")?
        .to_path_buf();

    let mut tests: TestFile = toml::from_slice(std::fs::read(&path)?.as_slice())?;
    crate::grint!(
        "Building",
        "`{}`",
        path.as_ref()
            .canonicalize()?
            .to_str()
            .context("Cannot translate path to string")?
    );

    let mut resolved_source_path = test_dir.clone();
    tests.tests.iter_mut().for_each(|x| {
        x.generator = x.generator.as_ref().map(|x| {
            let mut parent = resolved_source_path.clone();
            parent.push(x);
            parent
        })
    });

    resolved_source_path.push(tests.source_file);
    let binary = translator::translate(resolved_source_path)?;

    crate::grint!("Testing", "Running {} tests", tests.tests.len());

    for (n, test) in tests.tests.iter_mut().enumerate() {
        let name = if let Some(name) = test.name.clone() {
            name
        } else {
            n.to_string()
        };
        test.name = Some(name.clone());
        let start = Instant::now();
        match test.run(&binary, &test_dir) {
            Ok(()) => {
                crate::grint!(
                    "Success",
                    "Test {} completed in {:.5}s",
                    name,
                    start.elapsed().as_secs_f32()
                )
            }
            Err(report) => {
                crate::errint!("Failure!", "Test {}", name);
                println!("{}", report.root_cause());
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
