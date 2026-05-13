pub mod ast;
pub mod compiler;
pub mod parser;

use color_eyre::eyre;
use eyre::Result;

use rust_embed::RustEmbed;
use std::fs;
use std::path::Path;

#[derive(RustEmbed)]
#[folder = "std/"]
struct StdLibrary;

pub fn translate(input_path: impl AsRef<Path>) -> Result<Vec<u8>> {
    let program = load_module(input_path.as_ref())?;
    let mut compiler = compiler::Compiler::new();
    let binary = compiler.compile(program)?;
    Ok(binary)
}

fn load_module(file_path: &Path) -> Result<ast::Program> {
    let source = fs::read_to_string(file_path)?;
    let mut program = parser::parse(&source)?;
    let dir = file_path.parent().unwrap_or(Path::new(""));

    // Разрешаем модули пользовательского кода
    resolve_inline_modules(dir, &mut program.items)?;

    // Проверяем, не переопределил ли пользователь std
    let has_std = program.items.iter().any(|item| {
        if let ast::Item::ModDecl(m) = item {
            m.name == "std"
        } else {
            false
        }
    });

    // Если нет, автоматически внедряем встроенный
    if !has_std {
        let std_items = load_std()?;
        let std_mod = ast::Item::ModDecl(ast::ModDecl {
            vis: true,
            name: "std".to_string(),
            body: Some(std_items),
        });
        program.items.insert(0, std_mod);
    }

    Ok(program)
}

fn resolve_inline_modules(dir: &Path, items: &mut [ast::Item]) -> Result<()> {
    for item in items {
        if let ast::Item::ModDecl(m) = item {
            if let Some(mbody) = &mut m.body {
                resolve_inline_modules(dir.join(&m.name).as_path(), mbody)?;
            } else {
                let mod_file1 = dir.join(format!("{}.ferrite", m.name));
                let mod_file2 = dir.join(format!("{}/mod.ferrite", m.name));

                let actual_file = if mod_file1.exists() {
                    mod_file1
                } else if mod_file2.exists() {
                    mod_file2
                } else {
                    eyre::bail!("Module file not found for '{}'", m.name);
                };

                let sub_mod = load_module(&actual_file)?;
                m.body = Some(sub_mod.items);
            }
        }
    }
    Ok(())
}

fn load_std() -> Result<Vec<ast::Item>> {
    let file = StdLibrary::get("mod.ferrite")
        .ok_or_else(|| eyre::anyhow!("std/mod.ferrite not found in embedded files"))?;
    let source = std::str::from_utf8(file.data.as_ref())?;
    let mut program = parser::parse(source)?;
    resolve_embedded_modules("", &mut program.items)?;
    Ok(program.items)
}

fn resolve_embedded_modules(base_dir: &str, items: &mut [ast::Item]) -> Result<()> {
    for item in items {
        if let ast::Item::ModDecl(m) = item {
            if let Some(mbody) = &mut m.body {
                let next_base_dir = if base_dir.is_empty() {
                    m.name.clone()
                } else {
                    format!("{}/{}", base_dir, m.name)
                };
                resolve_embedded_modules(&next_base_dir, mbody)?;
            } else {
                let path1 = if base_dir.is_empty() {
                    format!("{}.ferrite", m.name)
                } else {
                    format!("{}/{}.ferrite", base_dir, m.name)
                };
                let path2 = if base_dir.is_empty() {
                    format!("{}/mod.ferrite", m.name)
                } else {
                    format!("{}/{}/mod.ferrite", base_dir, m.name)
                };

                let file = if let Some(f) = StdLibrary::get(&path1) {
                    f
                } else if let Some(f) = StdLibrary::get(&path2) {
                    f
                } else {
                    eyre::bail!("Embedded module file not found for '{}'", m.name);
                };

                let source = std::str::from_utf8(file.data.as_ref())?;
                let mut sub_mod = parser::parse(source)?;

                let next_base_dir = if base_dir.is_empty() {
                    m.name.clone()
                } else {
                    format!("{}/{}", base_dir, m.name)
                };
                resolve_embedded_modules(&next_base_dir, &mut sub_mod.items)?;
                m.body = Some(sub_mod.items);
            }
        }
    }
    Ok(())
}
