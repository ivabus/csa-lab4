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

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    fn fmt(source: &str) -> String {
        let program = parse(source).expect("parsing failed");
        format!("{:#?}", program)
    }

    #[test]
    fn test_ast_fn_no_args_no_return() {
        let s = fmt("fn main() { }");
        assert!(s.contains("FnDecl"));
        assert!(s.contains("main"));
    }

    #[test]
    fn test_ast_fn_with_args_and_return() {
        let s = fmt("fn add(a: i32, b: i32) -> i32 { return 0; }");
        assert!(s.contains("FnDecl"));
        assert!(s.contains("add"));
        assert!(s.contains("params"));
        assert!(s.contains("return_type"));
        assert!(s.contains("I32"));
    }

    #[test]
    fn test_ast_let_decl() {
        let s = fmt("fn main() { let x: i32 = 42; }");
        assert!(s.contains("LetDecl"));
        assert!(s.contains("x"));
        assert!(s.contains("I32"));
        assert!(s.contains("Literal"));
        assert!(s.contains("42"));
    }

    #[test]
    fn test_ast_let_mut() {
        let s = fmt("fn main() { let mut x = 10; }");
        assert!(s.contains("LetDecl"));
        assert!(s.contains("is_mut"));
    }

    #[test]
    fn test_ast_if_else() {
        let s = fmt("fn main() { if 1 { let a = 1; } else { let b = 2; } }");
        assert!(s.contains("IfStmt"));
        assert!(s.contains("condition"));
        assert!(s.contains("then_block"));
        assert!(s.contains("else_branch"));
        assert!(s.contains("Block"));
    }

    #[test]
    fn test_ast_if_elseif() {
        let s = fmt("fn main() { if 1 { } else if 2 { } else { } }");
        assert!(s.contains("IfStmt"));
        // else if creates a nested IfStmt via the ElseBranch::If variant
        assert!(s.contains("else_branch"));
    }

    #[test]
    fn test_ast_while() {
        let s = fmt("fn main() { while 1 { } }");
        assert!(s.contains("WhileStmt"));
        assert!(s.contains("condition"));
    }

    #[test]
    fn test_ast_break_continue() {
        let s = fmt("fn main() { while 1 { break; continue; } }");
        assert!(s.contains("Break"));
        assert!(s.contains("Continue"));
    }

    #[test]
    fn test_ast_return() {
        let s = fmt("fn main() -> i32 { return 42; }");
        assert!(s.contains("Return"));
        assert!(s.contains("42"));
    }

    #[test]
    fn test_ast_return_no_value() {
        let s = fmt("fn main() { return; }");
        assert!(s.contains("Return"));
        assert!(s.contains("None"));
    }

    #[test]
    fn test_ast_assign() {
        let s = fmt("fn main() { let mut x = 1; x = 2; }");
        assert!(s.contains("Assign"));
        assert!(s.contains("Assign"));
    }

    #[test]
    fn test_ast_assign_ops() {
        let s = fmt("fn main() { let mut x = 1; x += 2; x -= 3; x *= 4; x /= 5; x %= 6; x &= 7; x |= 8; x ^= 9; x <<= 2; x >>= 1; }");
        assert!(s.contains("AddEq"));
        assert!(s.contains("SubEq"));
        assert!(s.contains("MulEq"));
        assert!(s.contains("DivEq"));
        assert!(s.contains("ModEq"));
        assert!(s.contains("BitAndEq"));
        assert!(s.contains("BitOrEq"));
        assert!(s.contains("BitXorEq"));
        assert!(s.contains("ShlEq"));
        assert!(s.contains("ShrEq"));
    }

    #[test]
    fn test_ast_binary_ops() {
        let s = fmt("fn main() { let r = 1 + 2 - 3 * 4 / 5 % 6; }");
        assert!(s.contains("Binary"));
        assert!(s.contains("Add"));
        assert!(s.contains("Sub"));
        assert!(s.contains("Mul"));
        assert!(s.contains("Div"));
        assert!(s.contains("Mod"));
    }

    #[test]
    fn test_ast_bitwise_ops() {
        let s = fmt("fn main() { let r = 1 & 2 | 3 ^ 4; }");
        assert!(s.contains("BitAnd"));
        assert!(s.contains("BitOr"));
        assert!(s.contains("BitXor"));
    }

    #[test]
    fn test_ast_shift_ops() {
        let s = fmt("fn main() { let r = 1 << 2 >> 3; }");
        assert!(s.contains("Shl"));
        assert!(s.contains("Shr"));
    }

    #[test]
    fn test_ast_comparison_ops() {
        let s = fmt("fn main() { let r = 1 == 2 != 3 < 4 > 5 <= 6 >= 7; }");
        assert!(s.contains("Eq"));
        assert!(s.contains("Ne"));
        assert!(s.contains("Lt"));
        assert!(s.contains("Gt"));
        assert!(s.contains("Le"));
        assert!(s.contains("Ge"));
    }

    #[test]
    fn test_ast_logic_ops() {
        let s = fmt("fn main() { let r = true && false || true; }");
        assert!(s.contains("LogicAnd"));
        assert!(s.contains("LogicOr"));
    }

    #[test]
    fn test_ast_unary_neg() {
        let s = fmt("fn main() { let r = -1; }");
        assert!(s.contains("Unary"));
        assert!(s.contains("Neg"));
    }

    #[test]
    fn test_ast_unary_not() {
        let s = fmt("fn main() { let r = !true; }");
        assert!(s.contains("Not"));
    }

    #[test]
    fn test_ast_unary_bitnot() {
        let s = fmt("fn main() { let r = ~1; }");
        assert!(s.contains("BitNot"));
    }

    #[test]
    fn test_ast_literals() {
        let s = fmt(
            "fn main() { let a = 42; let b = 0xFF; let c = 'A'; let d = \"hello\"; let e = true; }",
        );
        assert!(s.contains("Literal"));
        assert!(s.contains("Int"));
        assert!(s.contains("Uint"));
        assert!(s.contains("Char"));
        assert!(s.contains("String"));
        assert!(s.contains("Bool"));
    }

    #[test]
    fn test_ast_call() {
        let s = fmt("fn main() { foo(1, 2); } fn foo(a: i32, b: i32) {}");
        assert!(s.contains("Call"));
        assert!(s.contains("foo"));
    }

    #[test]
    fn test_ast_index() {
        let s = fmt("fn main() { let arr: i32 = 0; let r = arr[0]; }");
        assert!(s.contains("Index"));
    }

    #[test]
    fn test_ast_reserve() {
        let s = fmt("fn main() { let buf = reserve::<i32>(10); }");
        assert!(s.contains("Reserve"));
        assert!(s.contains("I32"));
        assert!(s.contains("10"));
    }

    #[test]
    fn test_ast_const_decl() {
        let s = fmt("const MAX: i32 = 100; fn main() {}");
        assert!(s.contains("ConstDecl"));
        assert!(s.contains("MAX"));
        assert!(s.contains("I32"));
    }

    #[test]
    fn test_ast_pub_fn() {
        let s = fmt("pub fn main() {}");
        assert!(s.contains("vis"));
    }

    #[test]
    fn test_ast_mod_decl() {
        let s = fmt("mod foo; fn main() {}");
        assert!(s.contains("ModDecl"));
        assert!(s.contains("foo"));
    }

    #[test]
    fn test_ast_use_decl() {
        let s = fmt("use std::io; fn main() {}");
        assert!(s.contains("UseDecl"));
        assert!(s.contains("std"));
    }

    #[test]
    fn test_ast_use_glob() {
        let s = fmt("use std::*; fn main() {}");
        assert!(s.contains("Glob"));
    }

    #[test]
    fn test_ast_expr_stmt() {
        let s = fmt("fn main() { 42; }");
        assert!(s.contains("Expr"));
    }

    #[test]
    fn test_ast_assignment_deref() {
        let s = fmt("fn main() { let mut p: i32 = 0; *p = 42; }");
        assert!(s.contains("Deref"));
    }

    #[test]
    fn test_ast_assignment_index() {
        let s = fmt("fn main() { let mut arr: i32 = 0; arr[0] = 1; }");
        assert!(s.contains("Index"));
    }

    #[test]
    fn test_ast_hex_literal() {
        let s = fmt("fn main() { let r = 0xABCD; }");
        assert!(s.contains("Uint"));
        assert!(s.contains("43981")); // 0xABCD = 43981
    }

    #[test]
    fn test_ast_string_literal() {
        let s = fmt("fn main() { let s = \"hello world\"; }");
        assert!(s.contains("hello world"));
    }

    #[test]
    fn test_ast_types() {
        let s = fmt("fn main() { let a: i32 = 0; let b: u32 = 0; let c: i8 = 0; let d: u8 = 0; let e: bool = true; }");
        assert!(s.contains("I32"));
        assert!(s.contains("U32"));
        assert!(s.contains("I8"));
        assert!(s.contains("U8"));
        assert!(s.contains("Bool"));
    }

    #[test]
    fn test_ast_assignment_path() {
        let s = fmt("fn main() { let mut x = 0; x = x + 1; }");
        assert!(s.contains("Assign"));
        assert!(s.contains("Path"));
    }

    #[test]
    fn test_ast_use_nested() {
        let s = fmt("use std::io::{read, write}; fn main() {}");
        assert!(s.contains("Group"));
    }

    #[test]
    fn test_ast_while_local_scope() {
        let s = fmt("fn main() { while true { let x = 1; } }");
        assert!(s.contains("WhileStmt"));
        assert!(s.contains("LetDecl"));
        assert!(s.contains("Literal"));
    }

    #[test]
    fn test_ast_nested_blocks() {
        // Grammar doesn't support bare {} blocks; instead test a let in limited scope
        let s = fmt("fn main() { if true { let x = 1; } let y = 2; }");
        assert!(s.contains("IfStmt"));
        assert!(s.contains("LetDecl"));
    }

    #[test]
    fn test_ast_multiplication_and_addition() {
        let s = fmt("fn main() { let r = 2 * (3 + 4); }");
        assert!(s.contains("Binary"));
        assert!(s.contains("Mul"));
        assert!(s.contains("Add"));
    }
}
