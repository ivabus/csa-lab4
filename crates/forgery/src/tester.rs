use std::{
    collections::BTreeMap,
    io::Cursor,
    path::{Path, PathBuf},
    process::Stdio,
    time::Instant,
};

use color_eyre::eyre::{self, Context, ContextCompat};
use emulator::{IO, Processor};
use serde::{Deserialize, Serialize};

use crate::grint;

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
}

impl Test {
    fn run(&self, binary: &[u8]) -> eyre::Result<()> {
        for it in 0..self.iterations.unwrap_or(1) {
            let mut memory = BTreeMap::new();
            for (i, chunk) in binary.chunks(4).enumerate() {
                if chunk.len() == 4 {
                    let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    memory.insert(0x1000 + (i as u32 * 4), val);
                }
            }

            let mut out = Vec::new();
            let (input, asserted_output) = match (&self.input, &self.asserted_output) {
                (Some(input), Some(output)) => (input.as_bytes().to_vec(), output.clone()),
                (None, Some(output)) => (vec![], output.clone()),
                (None, None) if let Some(generator) = &self.generator => {
                    let child = std::process::Command::new(generator)
                        .stdout(Stdio::piped())
                        .spawn()?;
                    let out = child.wait_with_output()?.stdout;

                    #[derive(Deserialize)]
                    struct GeneratorResult {
                        input: String,
                        asserted_output: String,
                    }
                    let res: GeneratorResult =
                        toml::from_slice(&out).context("Generator failed to provide valid toml")?;

                    (res.input.as_bytes().to_vec(), res.asserted_output)
                }
                _ => eyre::bail!("Incorrect test format"),
            };

            let output = Cursor::new(&mut out);
            let mut io_map = BTreeMap::new();
            io_map.insert(0x0000_0000, IO::I(Box::new(input.as_slice())));
            io_map.insert(0x0000_0004, IO::O(Box::new(output)));

            let mut cpu = Processor::new(memory, io_map);
            cpu.trace = false;

            let mut limit = self.max_steps.unwrap_or(1_000_000_000);
            let start = Instant::now();
            while cpu.step() && limit > 0 {
                limit -= 1;
            }
            drop(cpu);
            let out = String::from_utf8(out)?;
            if out != *asserted_output {
                eyre::bail!(
                    "Assertion FAILED, left != right!\nleft (got): {}\nright (asserted): {}",
                    out.escape_debug(),
                    asserted_output.escape_debug()
                )
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
        Ok(())
    }
}

pub fn test(path: impl AsRef<Path>) -> eyre::Result<()> {
    let mut tests: TestFile = toml::from_slice(std::fs::read(&path)?.as_slice())?;
    crate::grint!(
        "Building",
        "`{}`",
        path.as_ref()
            .canonicalize()?
            .to_str()
            .context("Cannot translate path to string")?
    );
    let mut resolved_source_path = path
        .as_ref()
        .parent()
        .context("Failed to get parent dir")?
        .to_path_buf();
    tests.tests.iter_mut().for_each(|x| {
        x.generator = x.generator.as_ref().and_then(|x| {
            let mut parent = resolved_source_path.clone();
            parent.push(x);
            Some(parent)
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
        match test.run(&binary) {
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
