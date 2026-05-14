use crate::ast::*;
use shared::Opcode;
use std::collections::HashMap;

use color_eyre::eyre;

const CODE_BASE: usize = 0x1000;
const WORD_SIZE: usize = 4;

enum DataAlloc {
    String(String, u32),
    Reserve(Type, usize, u32),
}

pub struct Compiler {
    code: Vec<u32>,
    constants: HashMap<String, ConstantInfo>,
    functions: HashMap<String, FunctionInfo>,
    modules: HashMap<String, bool>,
    aliases: HashMap<String, String>,
    data_allocations: Vec<DataAlloc>,
    current_function: Option<FunctionContext>,
}

struct ConstantInfo {
    ty: Type,
    val: i32,
    is_pub: bool,
}

struct FunctionInfo {
    address: u32,
    _params: Vec<Param>,
    return_type: Option<Type>,
    is_pub: bool,
}

struct FunctionContext {
    info: FnDecl,
    current_mod: String,
    locals: Vec<LocalVar>,
    stack_depth: i32,
    loop_stack: Vec<LoopLabels>,
}

#[derive(Clone)]
struct LoopLabels {
    start_addr: u32,
    break_label_positions: Vec<usize>,
    locals_at_start: usize,
}

struct LocalVar {
    name: String,
    _ty: Type,
    _is_mut: bool,
    _frame_offset: i32,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: HashMap::new(),
            functions: HashMap::new(),
            modules: HashMap::new(),
            aliases: HashMap::new(),
            data_allocations: Vec::new(),
            current_function: None,
        }
    }

    pub fn compile(&mut self, program: Program) -> eyre::Result<Vec<u8>> {
        let uses = self.discover_symbols(&program.items, "crate".to_string())?;
        self.resolve_uses(uses)?;

        self.emit(Opcode::CallAddr as u32);
        let main_jump_pos = self.code.len();
        self.emit(0);
        self.emit(Opcode::Halt as u32);

        self.compile_items(&program.items, "crate".to_string())?;

        let main_info = self
            .functions
            .get("crate::main")
            .ok_or_else(|| eyre::eyre!("crate::main function not found"))?;

        self.code[main_jump_pos] = main_info.address;

        let mut binary = Vec::new();
        for word in &self.code {
            binary.extend_from_slice(&word.to_le_bytes());
        }

        let string_base = binary.len() as u32 + CODE_BASE as u32;
        let mut string_bytes = Vec::new();
        for alloc in &self.data_allocations {
            match alloc {
                DataAlloc::String(s, code_idx) => {
                    let addr = string_base + string_bytes.len() as u32;
                    self.code[*code_idx as usize] = addr;
                    string_bytes.extend_from_slice(s.as_bytes());
                    string_bytes.push(0);
                    while string_bytes.len() % WORD_SIZE != 0 {
                        string_bytes.push(0);
                    }
                }
                DataAlloc::Reserve(ty, count, code_idx) => {
                    let addr = string_base + string_bytes.len() as u32;
                    self.code[*code_idx as usize] = addr;
                    let bytes = match ty {
                        Type::I8 | Type::U8 | Type::Bool | Type::Cstr => *count,
                        _ => *count * WORD_SIZE,
                    };
                    string_bytes.resize(string_bytes.len() + bytes, 0);
                    while string_bytes.len() % WORD_SIZE != 0 {
                        string_bytes.push(0);
                    }
                }
            }
        }

        binary.clear();
        for word in &self.code {
            binary.extend_from_slice(&word.to_le_bytes());
        }
        binary.extend_from_slice(&string_bytes);

        Ok(binary)
    }

    fn discover_symbols(
        &mut self,
        items: &[Item],
        current_mod: String,
    ) -> eyre::Result<Vec<(UseDecl, String)>> {
        let mut uses = Vec::new();
        for item in items {
            match item {
                Item::ConstDecl(c) => {
                    let val = self.eval_const_expr(&c.value)?;
                    let fqn = format!("{}::{}", current_mod, c.name);
                    self.constants.insert(
                        fqn,
                        ConstantInfo {
                            ty: c.ty.clone(),
                            val,
                            is_pub: c.vis,
                        },
                    );
                }
                Item::FnDecl(f) => {
                    let fqn = format!("{}::{}", current_mod, f.name);
                    self.functions.insert(
                        fqn,
                        FunctionInfo {
                            address: 0,
                            _params: f.params.clone(),
                            return_type: f.return_type.clone(),
                            is_pub: f.vis,
                        },
                    );
                }
                Item::ModDecl(m) => {
                    let sub_mod = format!("{}::{}", current_mod, m.name);
                    self.modules.insert(sub_mod.clone(), m.vis);
                    if let Some(body) = &m.body {
                        uses.extend(self.discover_symbols(body, sub_mod)?);
                    }
                }
                Item::UseDecl(u) => {
                    uses.push((u.clone(), current_mod.clone()));
                }
            }
        }
        Ok(uses)
    }

    fn resolve_uses(&mut self, uses: Vec<(UseDecl, String)>) -> eyre::Result<()> {
        for (u, current_mod) in uses {
            self.expand_use_tree(&u.tree, None, &current_mod)?;
        }
        Ok(())
    }

    fn expand_use_tree(
        &mut self,
        tree: &UseTree,
        prefix: Option<Path>,
        current_mod: &str,
    ) -> eyre::Result<()> {
        match tree {
            UseTree::Path(p) => {
                let full_path = Self::combine_paths(prefix, p);
                let alias = full_path.segments.last().unwrap().clone();
                let fqn = format!("{}::{}", current_mod, alias);
                let resolved_target = self.resolve_path_for_use(&full_path, current_mod)?;
                self.aliases.insert(fqn, resolved_target);
            }
            UseTree::Glob => {
                let full_path = prefix.ok_or_else(|| eyre::eyre!("Cannot glob empty path"))?;
                let resolved_target = self.resolve_path_for_use(&full_path, current_mod)?;

                let prefix_str = format!("{}::", resolved_target);
                let mut to_alias = Vec::new();

                for (k, v) in &self.functions {
                    if k.starts_with(&prefix_str) && v.is_pub {
                        let name = k.strip_prefix(&prefix_str).unwrap();
                        if !name.contains("::") {
                            to_alias.push((name.to_string(), k.clone()));
                        }
                    }
                }
                for (k, v) in &self.constants {
                    if k.starts_with(&prefix_str) && v.is_pub {
                        let name = k.strip_prefix(&prefix_str).unwrap();
                        if !name.contains("::") {
                            to_alias.push((name.to_string(), k.clone()));
                        }
                    }
                }
                for (k, v) in &self.modules {
                    if k.starts_with(&prefix_str) && *v {
                        let name = k.strip_prefix(&prefix_str).unwrap();
                        if !name.contains("::") {
                            to_alias.push((name.to_string(), k.clone()));
                        }
                    }
                }

                for (name, target) in to_alias {
                    let fqn = format!("{}::{}", current_mod, name);
                    self.aliases.insert(fqn, target);
                }
            }
            UseTree::Group(trees) => {
                for t in trees {
                    self.expand_use_tree(t, prefix.clone(), current_mod)?;
                }
            }
            UseTree::Nested(p, inner) => {
                let full_path = Self::combine_paths(prefix, p);
                self.expand_use_tree(inner, Some(full_path), current_mod)?;
            }
        }
        Ok(())
    }

    fn combine_paths(prefix: Option<Path>, p: &Path) -> Path {
        if let Some(mut pre) = prefix {
            pre.segments.extend(p.segments.clone());
            pre
        } else {
            p.clone()
        }
    }

    fn resolve_path_for_use(&self, path: &Path, current_mod: &str) -> eyre::Result<String> {
        let raw_path = path.segments.join("::");
        if path.segments[0] == "crate" {
            return Ok(raw_path);
        }

        let root_fqn = format!("crate::{}", raw_path);
        if self.modules.contains_key(&root_fqn)
            || self.functions.contains_key(&root_fqn)
            || self.constants.contains_key(&root_fqn)
        {
            return Ok(root_fqn);
        }

        let local_fqn = format!("{}::{}", current_mod, path.segments[0]);
        let base_resolved = if let Some(target) = self.aliases.get(&local_fqn) {
            target.clone()
        } else {
            local_fqn
        };

        let mut resolved = base_resolved;
        if path.segments.len() > 1 {
            resolved.push_str("::");
            resolved.push_str(&path.segments[1..].join("::"));
        }

        Ok(resolved)
    }

    fn compile_items(&mut self, items: &[Item], current_mod: String) -> eyre::Result<()> {
        for item in items {
            match item {
                Item::FnDecl(f) => {
                    let fqn = format!("{}::{}", current_mod, f.name);
                    self.compile_function(f.clone(), fqn, current_mod.clone())?;
                }
                Item::ModDecl(m) => {
                    if let Some(body) = &m.body {
                        let sub_mod = format!("{}::{}", current_mod, m.name);
                        self.compile_items(body, sub_mod)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn resolve_path(&self, path: &Path, current_mod: &str) -> eyre::Result<String> {
        let raw_path = path.segments.join("::");
        if path.segments[0] == "crate" {
            return Ok(raw_path);
        }

        let local_fqn = format!("{}::{}", current_mod, path.segments[0]);
        let base_resolved = if let Some(target) = self.aliases.get(&local_fqn) {
            target.clone()
        } else {
            local_fqn
        };

        let mut resolved = base_resolved;
        if path.segments.len() > 1 {
            resolved.push_str("::");
            resolved.push_str(&path.segments[1..].join("::"));
        }

        if self.functions.contains_key(&resolved)
            || self.constants.contains_key(&resolved)
            || self.modules.contains_key(&resolved)
        {
            return Ok(resolved);
        }

        let root_fqn = format!("crate::{}", raw_path);
        if self.functions.contains_key(&root_fqn) || self.constants.contains_key(&root_fqn) {
            return Ok(root_fqn);
        }

        Err(eyre::eyre!("Could not resolve path: {}", raw_path))
    }

    fn check_visibility(&self, fqn: &str, current_mod: &str) -> eyre::Result<()> {
        let is_visible = if fqn.starts_with(current_mod)
            || current_mod.starts_with(&fqn[..fqn.rfind("::").unwrap_or(0)])
        {
            true
        } else {
            if let Some(f) = self.functions.get(fqn) {
                f.is_pub
            } else if let Some(c) = self.constants.get(fqn) {
                c.is_pub
            } else if let Some(is_pub) = self.modules.get(fqn) {
                *is_pub
            } else {
                false
            }
        };

        if !is_visible {
            eyre::bail!(
                "Item '{}' is private and not accessible from '{}'",
                fqn,
                current_mod
            );
        }
        Ok(())
    }

    fn compile_function(
        &mut self,
        f: FnDecl,
        fqn: String,
        current_mod: String,
    ) -> eyre::Result<()> {
        let addr = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
        self.functions.get_mut(&fqn).unwrap().address = addr;

        self.current_function = Some(FunctionContext {
            info: f.clone(),
            current_mod,
            locals: Vec::new(),
            stack_depth: 0,
            loop_stack: Vec::new(),
        });

        self.compile_block(f.body)?;

        if f.return_type.is_none() {
            let num_params = f.params.len();
            let num_locals = self.current_function.as_ref().unwrap().locals.len();
            for _ in 0..num_locals {
                self.emit(Opcode::Pop as u32);
            }
            if num_params == 1 {
                self.emit(Opcode::Wret as u32);
            } else if num_params == 0 {
                self.emit(Opcode::Ret as u32);
            } else {
                for _ in 0..num_params {
                    self.emit(Opcode::Swap as u32);
                    self.emit(Opcode::Pop as u32);
                }
                self.emit(Opcode::Ret as u32);
            }
        }

        self.current_function = None;
        Ok(())
    }

    fn compile_block(&mut self, block: Block) -> eyre::Result<()> {
        let initial_locals = self.current_function.as_ref().unwrap().locals.len();

        for stmt in block.statements {
            self.compile_statement(stmt)?;
        }

        let to_pop = self.current_function.as_mut().unwrap().locals.len() - initial_locals;
        for _ in 0..to_pop {
            self.current_function.as_mut().unwrap().locals.pop();
            self.emit(Opcode::Pop as u32);
        }

        Ok(())
    }

    fn compile_statement(&mut self, stmt: Statement) -> eyre::Result<()> {
        match stmt {
            Statement::LetDecl(l) => {
                let ty = self.compile_expression(l.value)?;
                let ctx = self.current_function.as_mut().unwrap();
                ctx.stack_depth -= 1;
                let frame_offset = ctx.locals.len() as i32 * WORD_SIZE as i32;
                ctx.locals.push(LocalVar {
                    name: l.name,
                    _ty: l.ty.unwrap_or(ty),
                    _is_mut: l.is_mut,
                    _frame_offset: frame_offset,
                });
            }
            Statement::IfStmt(i) => {
                self.compile_expression(i.condition)?;
                self.emit(Opcode::PushConst as u32);
                self.emit(0);
                self.emit(Opcode::JeqAddr as u32);
                let else_label_pos = self.code.len();
                self.emit(0);
                self.current_function.as_mut().unwrap().stack_depth -= 1;

                self.compile_block(i.then_block)?;

                if let Some(else_branch) = i.else_branch {
                    self.emit(Opcode::JumpAddr as u32);
                    let end_label_pos = self.code.len();
                    self.emit(0);

                    let else_addr = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                    self.code[else_label_pos] = else_addr;

                    match else_branch {
                        ElseBranch::Block(b) => self.compile_block(b)?,
                        ElseBranch::If(boxed_if) => {
                            self.compile_statement(Statement::IfStmt(*boxed_if))?
                        }
                    }

                    let end_addr = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                    self.code[end_label_pos] = end_addr;
                } else {
                    let end_addr = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                    self.code[else_label_pos] = end_addr;
                }
            }
            Statement::WhileStmt(w) => {
                let start_addr = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                let locals_at_start = self.current_function.as_ref().unwrap().locals.len();
                self.current_function
                    .as_mut()
                    .unwrap()
                    .loop_stack
                    .push(LoopLabels {
                        start_addr,
                        break_label_positions: Vec::new(),
                        locals_at_start,
                    });

                self.compile_expression(w.condition)?;
                self.emit(Opcode::PushConst as u32);
                self.emit(0);
                self.emit(Opcode::JeqAddr as u32);
                let end_label_pos = self.code.len();
                self.emit(0);
                self.current_function.as_mut().unwrap().stack_depth -= 1;

                self.compile_block(w.block)?;

                self.emit(Opcode::JumpAddr as u32);
                self.emit(start_addr);

                let end_addr = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                self.code[end_label_pos] = end_addr;

                let loop_info = self
                    .current_function
                    .as_mut()
                    .unwrap()
                    .loop_stack
                    .pop()
                    .unwrap();
                for pos in loop_info.break_label_positions {
                    self.code[pos] = end_addr;
                }
            }
            Statement::Break => {
                let ctx = self.current_function.as_ref().unwrap();
                let loop_info = ctx.loop_stack.last().unwrap();
                let to_pop = ctx.locals.len() - loop_info.locals_at_start;
                for _ in 0..to_pop {
                    self.emit(Opcode::Pop as u32);
                }

                self.emit(Opcode::JumpAddr as u32);
                let pos = self.code.len();
                self.emit(0);
                self.current_function
                    .as_mut()
                    .unwrap()
                    .loop_stack
                    .last_mut()
                    .unwrap()
                    .break_label_positions
                    .push(pos);
            }
            Statement::Continue => {
                let loop_info = self
                    .current_function
                    .as_ref()
                    .unwrap()
                    .loop_stack
                    .clone()
                    .last()
                    .unwrap()
                    .clone();
                let to_pop = self.current_function.as_mut().unwrap().locals.len()
                    - loop_info.locals_at_start;
                for _ in 0..to_pop {
                    self.emit(Opcode::Pop as u32);
                }

                let start_addr = loop_info.start_addr;
                self.emit(Opcode::JumpAddr as u32);
                self.emit(start_addr);
            }
            Statement::Return(val) => {
                if let Some(expr) = val {
                    self.compile_expression(expr)?;
                    let ctx = self.current_function.as_ref().unwrap();
                    let num_locals = ctx.locals.len();
                    let num_params = ctx.info.params.len();

                    for _ in 0..num_locals {
                        self.emit(Opcode::Swap as u32);
                        self.emit(Opcode::Pop as u32);
                    }
                    if num_params == 0 {
                        self.emit(Opcode::Swap as u32);
                        self.emit(Opcode::Ret as u32);
                    } else {
                        let offset = (num_params + 1) * WORD_SIZE;
                        self.emit(Opcode::StoreR as u32);
                        self.emit(offset as u32);
                        for _ in 0..num_params - 1 {
                            self.emit(Opcode::Swap as u32);
                            self.emit(Opcode::Pop as u32);
                        }
                        self.emit(Opcode::Ret as u32);
                    }
                    self.current_function.as_mut().unwrap().stack_depth -= 1;
                } else {
                    let (num_locals, num_params) = {
                        let ctx = self.current_function.as_ref().unwrap();
                        (ctx.locals.len(), ctx.info.params.len())
                    };
                    for _ in 0..num_locals {
                        self.emit(Opcode::Pop as u32);
                    }
                    if num_params == 1 {
                        self.emit(Opcode::Wret as u32);
                    } else if num_params == 0 {
                        self.emit(Opcode::Ret as u32);
                    } else {
                        for _ in 0..num_params {
                            self.emit(Opcode::Swap as u32);
                            self.emit(Opcode::Pop as u32);
                        }
                        self.emit(Opcode::Ret as u32);
                    }
                }
            }
            Statement::Assign(a) => {
                match a.target {
                    AssignTarget::Path(ref path) => {
                        let name = &path.segments[0];
                        let mut ty = Type::I32;

                        if a.op != AssignOp::Assign {
                            let (offset, local_ty) = self.get_local_info(name)?;
                            ty = local_ty;
                            self.emit(Opcode::PushR as u32);
                            self.emit(offset as u32);
                            self.current_function.as_mut().unwrap().stack_depth += 1;
                        }

                        self.compile_expression(a.value)?;

                        if a.op != AssignOp::Assign {
                            let opcode = match a.op {
                                AssignOp::ShlEq => Opcode::Ls,
                                AssignOp::ShrEq => match ty {
                                    Type::I32 | Type::I8 => Opcode::Ars,
                                    _ => Opcode::Rs,
                                },
                                _ => a.op.to_opcode().unwrap(),
                            };
                            self.emit(opcode as u32);
                            self.current_function.as_mut().unwrap().stack_depth -= 1;
                        }

                        let (offset_store, _) = self.get_local_info(name)?;

                        self.emit(Opcode::StoreR as u32);
                        self.emit(offset_store as u32);
                        self.current_function.as_mut().unwrap().stack_depth -= 1;
                    }
                    AssignTarget::Deref(ref addr_expr) => {
                        let ptr_ty = self.compile_expression(addr_expr.clone())?; // [addr]
                        let elem_ty = match ptr_ty {
                            Type::Cstr => Type::U8,
                            _ => Type::I32,
                        };
                        if a.op != AssignOp::Assign {
                            self.emit(Opcode::Dup as u32); // [addr, addr]
                            self.emit_smc_load(elem_ty.clone())?; // [addr, old_val]
                            self.current_function.as_mut().unwrap().stack_depth += 1;
                            let _val_ty = self.compile_expression(a.value)?; // [addr, old_val, rhs_val]

                            let opcode = match a.op {
                                AssignOp::ShlEq => Opcode::Ls,
                                AssignOp::ShrEq => Opcode::Ars,
                                _ => a.op.to_opcode().unwrap(),
                            };
                            self.emit(opcode as u32); // [addr, new_val]
                            self.current_function.as_mut().unwrap().stack_depth -= 1;
                            self.emit(Opcode::Swap as u32); // [new_val, addr]
                            self.emit_smc_store(elem_ty)?;
                            self.current_function.as_mut().unwrap().stack_depth -= 2;
                        } else {
                            let _val_ty = self.compile_expression(a.value)?; // [addr, val]
                            self.emit(Opcode::Swap as u32); // [val, addr]
                            self.emit_smc_store(elem_ty)?;
                            self.current_function.as_mut().unwrap().stack_depth -= 2;
                        }
                    }
                    AssignTarget::Index(ref path, ref idx_expr) => {
                        let ty = if path.segments.len() == 1
                            && self.get_local_info(&path.segments[0]).is_ok()
                        {
                            let (offset, ty) = self.get_local_info(&path.segments[0]).unwrap();
                            self.emit(Opcode::PushR as u32);
                            self.emit(offset as u32);
                            self.current_function.as_mut().unwrap().stack_depth += 1;
                            ty
                        } else {
                            let ctx = self.current_function.as_ref().unwrap();
                            let fqn = self.resolve_path(path, &ctx.current_mod)?;
                            self.check_visibility(&fqn, &ctx.current_mod)?;

                            let (c_ty, c_val) = {
                                let c = self.constants.get(&fqn).unwrap();
                                (c.ty.clone(), c.val)
                            };

                            self.emit(Opcode::PushConst as u32);
                            self.emit(c_val as u32);
                            self.current_function.as_mut().unwrap().stack_depth += 1;
                            c_ty
                        };

                        let elem_ty = match ty {
                            Type::Cstr => Type::U8,
                            _ => Type::I32,
                        };

                        self.compile_expression(idx_expr.clone())?;

                        // Если это массив слов/dwords, масштабируем индекс в байтовое смещение
                        if elem_ty == Type::I32 {
                            self.emit(Opcode::PushConst as u32);
                            self.emit(WORD_SIZE as u32);
                            self.emit(Opcode::Mul as u32);
                        }

                        self.emit(Opcode::Add as u32);
                        self.current_function.as_mut().unwrap().stack_depth -= 1; // [addr]

                        if a.op != AssignOp::Assign {
                            self.emit(Opcode::Dup as u32);
                            self.emit_smc_load(elem_ty.clone())?; // [addr, old_val]
                            self.current_function.as_mut().unwrap().stack_depth += 1;
                            let _val_ty = self.compile_expression(a.value)?; // [addr, old_val, rhs_val]

                            let opcode = match a.op {
                                AssignOp::ShlEq => Opcode::Ls,
                                AssignOp::ShrEq => match elem_ty {
                                    Type::I32 | Type::I8 => Opcode::Ars,
                                    _ => Opcode::Rs,
                                },
                                _ => a.op.to_opcode().unwrap(),
                            };
                            self.emit(opcode as u32); // [addr, new_val]
                            self.current_function.as_mut().unwrap().stack_depth -= 1;
                            self.emit(Opcode::Swap as u32); // [new_val, addr]
                            self.emit_smc_store(elem_ty)?;
                            self.current_function.as_mut().unwrap().stack_depth -= 2;
                        } else {
                            let _val_ty = self.compile_expression(a.value)?; // [addr, val]
                            self.emit(Opcode::Swap as u32); // [val, addr]
                            self.emit_smc_store(elem_ty)?;
                            self.current_function.as_mut().unwrap().stack_depth -= 2;
                        }
                    }
                }
            }
            Statement::Expr(e) => {
                self.compile_expression(e)?;
                self.emit(Opcode::Pop as u32);
                self.current_function.as_mut().unwrap().stack_depth -= 1;
            }
        }
        Ok(())
    }

    fn compile_expression(&mut self, expr: Expression) -> eyre::Result<Type> {
        match expr {
            Expression::Literal(l) => {
                self.emit(Opcode::PushConst as u32);
                let ty = match l {
                    Literal::Int(v) => {
                        self.emit(v as u32);
                        Type::I32
                    }
                    Literal::Uint(v) => {
                        self.emit(v);
                        Type::U32
                    }
                    Literal::Char(v) => {
                        self.emit(v as u32);
                        Type::U8
                    }
                    Literal::String(s) => {
                        self.emit(0);
                        self.data_allocations
                            .push(DataAlloc::String(s, self.code.len() as u32 - 1));
                        Type::Cstr
                    }
                    Literal::Bool(b) => {
                        self.emit(if b { 1 } else { 0 });
                        Type::Bool
                    }
                };
                self.current_function.as_mut().unwrap().stack_depth += 1;
                Ok(ty)
            }
            Expression::Reserve(ty, count) => {
                self.emit(Opcode::PushConst as u32);
                self.emit(0);
                self.data_allocations.push(DataAlloc::Reserve(
                    ty.clone(),
                    count,
                    self.code.len() as u32 - 1,
                ));
                self.current_function.as_mut().unwrap().stack_depth += 1;
                match ty {
                    Type::U8 | Type::I8 => Ok(Type::Cstr),
                    _ => Ok(Type::I32),
                }
            }
            Expression::Path(path) => {
                if path.segments.len() == 1 {
                    if let Ok((offset, ty)) = self.get_local_info(&path.segments[0]) {
                        self.emit(Opcode::PushR as u32);
                        self.emit(offset as u32);
                        self.current_function.as_mut().unwrap().stack_depth += 1;
                        return Ok(ty);
                    }
                }

                let ctx = self.current_function.as_ref().unwrap();
                let fqn = self.resolve_path(&path, &ctx.current_mod)?;
                self.check_visibility(&fqn, &ctx.current_mod)?;

                let c_info = self.constants.get(&fqn).map(|c| (c.ty.clone(), c.val));
                if let Some((ty, val)) = c_info {
                    self.emit(Opcode::PushConst as u32);
                    self.emit(val as u32);
                    self.current_function.as_mut().unwrap().stack_depth += 1;
                    Ok(ty)
                } else {
                    Err(eyre::eyre!(
                        "Undefined identifier or constant: {}",
                        path.segments.join("::")
                    ))
                }
            }
            Expression::Binary(op, left, right) => {
                if op == BinaryOp::LogicAnd {
                    self.compile_expression(*left)?;
                    self.emit(Opcode::Dup as u32);
                    self.emit(Opcode::PushConst as u32);
                    self.emit(0);
                    self.emit(Opcode::JeqAddr as u32);
                    let false_label_pos = self.code.len();
                    self.emit(0);
                    self.emit(Opcode::Pop as u32);
                    self.current_function.as_mut().unwrap().stack_depth -= 1;
                    self.compile_expression(*right)?;
                    self.emit(Opcode::JumpAddr as u32);
                    let end_label_pos = self.code.len();
                    self.emit(0);
                    self.code[false_label_pos] = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                    self.code[end_label_pos] = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                    return Ok(Type::Bool);
                }
                if op == BinaryOp::LogicOr {
                    self.compile_expression(*left)?;
                    self.emit(Opcode::Dup as u32);
                    self.emit(Opcode::PushConst as u32);
                    self.emit(0);
                    self.emit(Opcode::JneAddr as u32);
                    let true_label_pos = self.code.len();
                    self.emit(0);
                    self.emit(Opcode::Pop as u32);
                    self.current_function.as_mut().unwrap().stack_depth -= 1;
                    self.compile_expression(*right)?;
                    self.emit(Opcode::JumpAddr as u32);
                    let end_label_pos = self.code.len();
                    self.emit(0);
                    self.code[true_label_pos] = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                    self.emit(Opcode::Pop as u32);
                    self.emit(Opcode::PushConst as u32);
                    self.emit(1);
                    self.code[end_label_pos] = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
                    return Ok(Type::Bool);
                }

                let left_ty = self.compile_expression(*left)?;
                let _right_ty = self.compile_expression(*right)?;
                let res_ty = match op {
                    BinaryOp::Add => {
                        self.emit(Opcode::Add as u32);
                        left_ty
                    }
                    BinaryOp::Sub => {
                        self.emit(Opcode::Sub as u32);
                        left_ty
                    }
                    BinaryOp::Mul => {
                        self.emit(Opcode::Mul as u32);
                        left_ty
                    }
                    BinaryOp::Div => {
                        self.emit(Opcode::Div as u32);
                        left_ty
                    }
                    BinaryOp::Mod => {
                        self.emit(Opcode::Mod as u32);
                        left_ty
                    }
                    BinaryOp::BitAnd => {
                        self.emit(Opcode::And as u32);
                        left_ty
                    }
                    BinaryOp::BitOr => {
                        self.emit(Opcode::Or as u32);
                        left_ty
                    }
                    BinaryOp::BitXor => {
                        self.emit(Opcode::Xor as u32);
                        left_ty
                    }
                    BinaryOp::Shl => {
                        self.emit(Opcode::Ls as u32);
                        left_ty
                    }
                    BinaryOp::Shr => {
                        match left_ty {
                            Type::I32 | Type::I8 => self.emit(Opcode::Ars as u32),
                            _ => self.emit(Opcode::Rs as u32),
                        }
                        left_ty
                    }
                    BinaryOp::Eq => {
                        self.emit_comparison(Opcode::JeqAddr);
                        Type::Bool
                    }
                    BinaryOp::Ne => {
                        self.emit_comparison(Opcode::JneAddr);
                        Type::Bool
                    }
                    BinaryOp::Lt => {
                        self.emit_comparison(Opcode::JltAddr);
                        Type::Bool
                    }
                    BinaryOp::Gt => {
                        self.emit_comparison(Opcode::JgtAddr);
                        Type::Bool
                    }
                    BinaryOp::Le => {
                        self.emit_comparison(Opcode::JleAddr);
                        Type::Bool
                    }
                    BinaryOp::Ge => {
                        self.emit_comparison(Opcode::JgeAddr);
                        Type::Bool
                    }
                    _ => unreachable!(),
                };
                self.current_function.as_mut().unwrap().stack_depth -= 1;
                Ok(res_ty)
            }
            Expression::Unary(op, inner) => match op {
                UnaryOp::Deref => {
                    let inner_ty = self.compile_expression(*inner)?;
                    let res_ty = match inner_ty {
                        Type::Cstr => Type::U8,
                        _ => Type::I32,
                    };
                    self.emit_smc_load(res_ty.clone())?;
                    Ok(res_ty)
                }
                UnaryOp::Neg => {
                    self.emit(Opcode::PushConst as u32);
                    self.emit(0);
                    self.current_function.as_mut().unwrap().stack_depth += 1;
                    let ty = self.compile_expression(*inner)?;
                    self.emit(Opcode::Sub as u32);
                    self.current_function.as_mut().unwrap().stack_depth -= 1;
                    Ok(ty)
                }
                UnaryOp::Not | UnaryOp::BitNot => {
                    let ty = self.compile_expression(*inner)?;
                    self.emit(Opcode::Not as u32);
                    Ok(ty)
                }
            },
            Expression::Call(path, args) => {
                let num_args = args.len();
                for arg in args {
                    self.compile_expression(arg)?;
                }

                let ctx = self.current_function.as_ref().unwrap();
                let fqn = self.resolve_path(&path, &ctx.current_mod)?;
                self.check_visibility(&fqn, &ctx.current_mod)?;

                let (addr, return_type) = {
                    let info = self
                        .functions
                        .get(&fqn)
                        .ok_or_else(|| eyre::eyre!("Undefined function: {}", fqn))?;
                    (info.address, info.return_type.clone())
                };

                self.emit(Opcode::CallAddr as u32);
                self.emit(addr);
                self.current_function.as_mut().unwrap().stack_depth -= num_args as i32;

                if return_type.is_none() {
                    self.emit(Opcode::PushConst as u32);
                    self.emit(0);
                }

                self.current_function.as_mut().unwrap().stack_depth += 1;
                Ok(return_type.unwrap_or(Type::I32))
            }
            Expression::Index(path, idx_expr) => {
                let ty =
                    if path.segments.len() == 1 && self.get_local_info(&path.segments[0]).is_ok() {
                        let (offset, ty) = self.get_local_info(&path.segments[0]).unwrap();
                        self.emit(Opcode::PushR as u32);
                        self.emit(offset as u32);
                        self.current_function.as_mut().unwrap().stack_depth += 1;
                        ty
                    } else {
                        let ctx = self.current_function.as_ref().unwrap();
                        let fqn = self.resolve_path(&path, &ctx.current_mod)?;
                        self.check_visibility(&fqn, &ctx.current_mod)?;

                        let (c_ty, c_val) = {
                            let c = self.constants.get(&fqn).unwrap();
                            (c.ty.clone(), c.val)
                        };
                        self.emit(Opcode::PushConst as u32);
                        self.emit(c_val as u32);
                        self.current_function.as_mut().unwrap().stack_depth += 1;
                        c_ty
                    };

                self.compile_expression(*idx_expr)?;

                let res_ty = match ty {
                    Type::Cstr => Type::U8,
                    _ => Type::I32,
                };

                if res_ty == Type::I32 {
                    self.emit(Opcode::PushConst as u32);
                    self.emit(WORD_SIZE as u32);
                    self.emit(Opcode::Mul as u32);
                }

                self.emit(Opcode::Add as u32);
                self.current_function.as_mut().unwrap().stack_depth -= 1;

                self.emit_smc_load(res_ty.clone())?;
                Ok(res_ty)
            }
        }
    }

    fn emit(&mut self, val: u32) {
        self.code.push(val);
    }

    fn emit_comparison(&mut self, jump_op: Opcode) {
        self.emit(jump_op as u32);
        let true_label = self.code.len();
        self.emit(0);
        self.emit(Opcode::PushConst as u32);
        self.emit(0);
        self.emit(Opcode::JumpAddr as u32);
        let end_label = self.code.len();
        self.emit(0);
        self.code[true_label] = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
        self.emit(Opcode::PushConst as u32);
        self.emit(1);
        self.code[end_label] = (self.code.len() * WORD_SIZE + CODE_BASE) as u32;
    }

    fn emit_smc_load(&mut self, ty: Type) -> eyre::Result<()> {
        match ty {
            Type::I8 | Type::U8 => {
                self.emit(Opcode::Dup as u32);
                self.emit(Opcode::PushConst as u32);
                self.emit(3);
                self.emit(Opcode::And as u32);
                self.emit(Opcode::Swap as u32);

                let store_addr_pos = self.code.len();
                self.emit(Opcode::StoreAddr as u32);
                self.emit(0);
                let push_addr_pos = self.code.len();
                self.emit(Opcode::PushAddr as u32);
                self.emit(0);

                self.code[store_addr_pos + 1] =
                    (push_addr_pos * WORD_SIZE + WORD_SIZE + CODE_BASE) as u32;

                self.emit(Opcode::Swap as u32);
                self.emit(Opcode::PushConst as u32);
                self.emit(8);
                self.emit(Opcode::Mul as u32);

                match ty {
                    Type::I8 => self.emit(Opcode::Ars as u32),
                    _ => self.emit(Opcode::Rs as u32),
                }

                self.emit(Opcode::PushConst as u32);
                self.emit(0xFF);
                self.emit(Opcode::And as u32);

                if ty == Type::I8 {
                    self.emit(Opcode::PushConst as u32);
                    self.emit(24);
                    self.emit(Opcode::Ls as u32);
                    self.emit(Opcode::PushConst as u32);
                    self.emit(24);
                    self.emit(Opcode::Ars as u32);
                }
            }
            _ => {
                let store_addr_pos = self.code.len();
                self.emit(Opcode::StoreAddr as u32);
                self.emit(0);
                let push_addr_pos = self.code.len();
                self.emit(Opcode::PushAddr as u32);
                self.emit(0);
                self.code[store_addr_pos + 1] =
                    (push_addr_pos * WORD_SIZE + WORD_SIZE + CODE_BASE) as u32;
            }
        }
        Ok(())
    }

    fn emit_smc_store(&mut self, ty: Type) -> eyre::Result<()> {
        match ty {
            Type::I8 | Type::U8 => {
                self.emit(Opcode::Dup as u32);
                self.emit(Opcode::PushConst as u32);
                self.emit(3);
                self.emit(Opcode::And as u32);
                self.emit(Opcode::PushConst as u32);
                self.emit(8);
                self.emit(Opcode::Mul as u32);
                self.emit(Opcode::Over as u32);

                let store_addr_load_pos = self.code.len();
                self.emit(Opcode::StoreAddr as u32);
                self.emit(0);
                let push_addr_pos = self.code.len();
                self.emit(Opcode::PushAddr as u32);
                self.emit(0);
                self.code[store_addr_load_pos + 1] =
                    (push_addr_pos * WORD_SIZE + WORD_SIZE + CODE_BASE) as u32;

                self.emit(Opcode::PushConst as u32);
                self.emit(0xFF);
                self.emit(Opcode::PushR as u32);
                self.emit(8);
                self.emit(Opcode::Ls as u32);
                self.emit(Opcode::Not as u32);
                self.emit(Opcode::And as u32);
                self.emit(Opcode::PushR as u32);
                self.emit(12);
                self.emit(Opcode::PushConst as u32);
                self.emit(0xFF);
                self.emit(Opcode::And as u32);
                self.emit(Opcode::PushR as u32);
                self.emit(8);
                self.emit(Opcode::Ls as u32);
                self.emit(Opcode::Or as u32);
                self.emit(Opcode::PushR as u32);
                self.emit(8);

                let store_addr_store_pos = self.code.len();
                self.emit(Opcode::StoreAddr as u32);
                self.emit(0);
                let store_addr_final_pos = self.code.len();
                self.emit(Opcode::StoreAddr as u32);
                self.emit(0);
                self.code[store_addr_store_pos + 1] =
                    (store_addr_final_pos * WORD_SIZE + WORD_SIZE + CODE_BASE) as u32;

                self.emit(Opcode::Pop as u32);
                self.emit(Opcode::Pop as u32);
                self.emit(Opcode::Pop as u32);
            }
            _ => {
                self.emit(Opcode::Dup as u32);
                let store_addr_patch_pos = self.code.len();
                self.emit(Opcode::StoreAddr as u32);
                self.emit(0);
                self.emit(Opcode::Pop as u32);
                let store_addr_final_pos = self.code.len();
                self.emit(Opcode::StoreAddr as u32);
                self.emit(0);
                self.code[store_addr_patch_pos + 1] =
                    (store_addr_final_pos * WORD_SIZE + WORD_SIZE + CODE_BASE) as u32;
            }
        }
        Ok(())
    }

    fn get_local_info(&self, name: &str) -> eyre::Result<(i32, Type)> {
        let ctx = self.current_function.as_ref().unwrap();
        if let Some(pos) = ctx.locals.iter().position(|l| l.name == name) {
            let num_locals = ctx.locals.len();
            let num_temps = ctx.stack_depth;
            let items_above = (num_locals - 1 - pos) as i32 + num_temps;
            return Ok((items_above * WORD_SIZE as i32, ctx.locals[pos]._ty.clone()));
        }
        if let Some(pos) = ctx.info.params.iter().position(|p| p.name == name) {
            let num_locals = ctx.locals.len();
            let num_temps = ctx.stack_depth;
            let num_params = ctx.info.params.len();
            let items_above = (num_params - 1 - pos) as i32 + 1 + num_locals as i32 + num_temps;
            return Ok((
                items_above * WORD_SIZE as i32,
                ctx.info.params[pos].ty.clone(),
            ));
        }
        Err(eyre::eyre!("Local not found: {}", name))
    }

    fn eval_const_expr(&self, expr: &Expression) -> eyre::Result<i32> {
        match expr {
            Expression::Literal(Literal::Int(v)) => Ok(*v),
            Expression::Literal(Literal::Uint(v)) => Ok(*v as i32),
            _ => Err(eyre::eyre!(
                "Only integer literals are supported in constants"
            )),
        }
    }
}
