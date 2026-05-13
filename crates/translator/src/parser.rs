use crate::ast::*;
use color_eyre::eyre;
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "ferrite.pest"]
pub struct FerriteParser;

pub fn parse(source: &str) -> eyre::Result<Program> {
    let pairs = FerriteParser::parse(Rule::program, source)?;
    let mut items = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::program => {
                for item_pair in pair.into_inner() {
                    if item_pair.as_rule() == Rule::item {
                        items.push(parse_item(item_pair.into_inner().next().unwrap())?);
                    }
                }
            }
            Rule::EOI => (),
            _ => unreachable!("Unexpected rule: {:?}", pair.as_rule()),
        }
    }

    Ok(Program { items })
}

fn parse_item(pair: Pair<Rule>) -> eyre::Result<Item> {
    match pair.as_rule() {
        Rule::const_decl => Ok(Item::ConstDecl(parse_const_decl(pair)?)),
        Rule::fn_decl => Ok(Item::FnDecl(parse_fn_decl(pair)?)),
        Rule::mod_decl => Ok(Item::ModDecl(parse_mod_decl(pair)?)),
        Rule::use_decl => Ok(Item::UseDecl(parse_use_decl(pair)?)),
        _ => unreachable!(),
    }
}

fn parse_vis(pair: Pair<Rule>) -> bool {
    pair.as_str() == "pub"
}

fn parse_path(pair: Pair<Rule>) -> eyre::Result<Path> {
    let segments = pair.into_inner().map(|p| p.as_str().to_string()).collect();
    Ok(Path { segments })
}

fn parse_mod_decl(pair: Pair<Rule>) -> eyre::Result<ModDecl> {
    let mut inner = pair.into_inner();
    let vis = parse_vis(inner.next().unwrap());
    let name = inner.next().unwrap().as_str().to_string();
    let body = if let Some(block_pair) = inner.next() {
        let mut items = Vec::new();
        for item_pair in block_pair.into_inner() {
            items.push(parse_item(item_pair)?);
        }
        Some(items)
    } else {
        None
    };
    Ok(ModDecl { vis, name, body })
}

fn parse_use_decl(pair: Pair<Rule>) -> eyre::Result<UseDecl> {
    let mut inner = pair.into_inner();
    let vis = parse_vis(inner.next().unwrap());
    let tree = parse_use_tree(inner.next().unwrap())?;
    Ok(UseDecl { vis, tree })
}

fn parse_use_tree(pair: Pair<Rule>) -> eyre::Result<UseTree> {
    match pair.as_rule() {
        Rule::use_tree => parse_use_tree(pair.into_inner().next().unwrap()),
        Rule::use_path_with_tree => {
            let mut inner = pair.into_inner();
            let path = parse_path(inner.next().unwrap())?;
            let suffix_pair = inner.next().unwrap();
            let suffix = match suffix_pair.as_rule() {
                Rule::glob => UseTree::Glob,
                Rule::use_group => parse_use_group(suffix_pair)?,
                _ => unreachable!(),
            };
            Ok(UseTree::Nested(path, Box::new(suffix)))
        }
        Rule::path => Ok(UseTree::Path(parse_path(pair)?)),
        Rule::glob => Ok(UseTree::Glob),
        Rule::use_group => parse_use_group(pair),
        _ => unreachable!("{:?}", pair.as_rule()),
    }
}

fn parse_use_group(pair: Pair<Rule>) -> eyre::Result<UseTree> {
    let mut trees = Vec::new();
    for tree_pair in pair.into_inner() {
        trees.push(parse_use_tree(tree_pair)?);
    }
    Ok(UseTree::Group(trees))
}

fn parse_const_decl(pair: Pair<Rule>) -> eyre::Result<ConstDecl> {
    let mut inner = pair.into_inner();
    let vis = parse_vis(inner.next().unwrap());
    let name = inner.next().unwrap().as_str().to_string();
    let ty = parse_type(inner.next().unwrap())?;
    let value = parse_expr(inner.next().unwrap())?;
    Ok(ConstDecl {
        vis,
        name,
        ty,
        value,
    })
}

fn parse_fn_decl(pair: Pair<Rule>) -> eyre::Result<FnDecl> {
    let mut inner = pair.into_inner();
    let vis = parse_vis(inner.next().unwrap());
    let name = inner.next().unwrap().as_str().to_string();
    let mut params = Vec::new();
    let mut return_type = None;

    let mut next = inner.next().unwrap();
    if next.as_rule() == Rule::params {
        for param_pair in next.into_inner() {
            let mut p_inner = param_pair.into_inner();
            params.push(Param {
                name: p_inner.next().unwrap().as_str().to_string(),
                ty: parse_type(p_inner.next().unwrap())?,
            });
        }
        next = inner.next().unwrap();
    }

    if next.as_rule() == Rule::type_name {
        return_type = Some(parse_type(next)?);
        next = inner.next().unwrap();
    }

    let body = parse_block(next)?;

    Ok(FnDecl {
        vis,
        name,
        params,
        return_type,
        body,
    })
}

fn parse_block(pair: Pair<Rule>) -> eyre::Result<Block> {
    let mut statements = Vec::new();
    for stmt_pair in pair.into_inner() {
        statements.push(parse_statement(stmt_pair)?);
    }
    Ok(Block { statements })
}

fn parse_statement(pair: Pair<Rule>) -> eyre::Result<Statement> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::let_decl => {
            let mut l_inner = inner.into_inner();
            let mut next = l_inner.next().unwrap();
            let is_mut = if next.as_str() == "mut" {
                next = l_inner.next().unwrap();
                true
            } else {
                false
            };
            let name = next.as_str().to_string();
            let mut next = l_inner.next().unwrap();
            let ty = if next.as_rule() == Rule::type_name {
                let t = Some(parse_type(next)?);
                next = l_inner.next().unwrap();
                t
            } else {
                None
            };
            let value = parse_expr(next)?;
            Ok(Statement::LetDecl(LetDecl {
                is_mut,
                name,
                ty,
                value,
            }))
        }
        Rule::if_stmt => Ok(Statement::IfStmt(parse_if_stmt(inner)?)),
        Rule::while_stmt => {
            let mut w_inner = inner.into_inner();
            let condition = parse_expr(w_inner.next().unwrap())?;
            let block = parse_block(w_inner.next().unwrap())?;
            Ok(Statement::WhileStmt(WhileStmt { condition, block }))
        }
        Rule::break_stmt => Ok(Statement::Break),
        Rule::continue_stmt => Ok(Statement::Continue),
        Rule::return_stmt => {
            let mut r_inner = inner.into_inner();
            let value = r_inner.next().map(parse_expr).transpose()?;
            Ok(Statement::Return(value))
        }
        Rule::assign_stmt => {
            let mut a_inner = inner.into_inner();
            let target_pair = a_inner.next().unwrap();
            let target = match target_pair.as_rule() {
                Rule::path => AssignTarget::Path(parse_path(target_pair)?),
                Rule::deref_target => {
                    AssignTarget::Deref(parse_expr(target_pair.into_inner().next().unwrap())?)
                }
                Rule::index_expr => {
                    let mut idx_inner = target_pair.into_inner();
                    let path = parse_path(idx_inner.next().unwrap())?;
                    let idx = parse_expr(idx_inner.next().unwrap())?;
                    AssignTarget::Index(path, idx)
                }
                _ => unreachable!(),
            };
            let op_pair = a_inner.next().unwrap();
            let op = match op_pair.as_str() {
                "=" => AssignOp::Assign,
                "+=" => AssignOp::AddEq,
                "-=" => AssignOp::SubEq,
                "*=" => AssignOp::MulEq,
                "/=" => AssignOp::DivEq,
                "%=" => AssignOp::ModEq,
                "&=" => AssignOp::BitAndEq,
                "|=" => AssignOp::BitOrEq,
                "^=" => AssignOp::BitXorEq,
                "<<=" => AssignOp::ShlEq,
                ">>=" => AssignOp::ShrEq,
                _ => unreachable!(),
            };
            let value = parse_expr(a_inner.next().unwrap())?;
            Ok(Statement::Assign(AssignStmt { target, op, value }))
        }
        Rule::expr_stmt => Ok(Statement::Expr(parse_expr(
            inner.into_inner().next().unwrap(),
        )?)),
        _ => unreachable!("parse_statement: {:?}", inner.as_rule()),
    }
}

fn parse_if_stmt(pair: Pair<Rule>) -> eyre::Result<IfStmt> {
    let mut inner = pair.into_inner();
    let condition = parse_expr(inner.next().unwrap())?;
    let then_block = parse_block(inner.next().unwrap())?;
    let else_branch = inner
        .next()
        .map(|p| {
            if p.as_rule() == Rule::if_stmt {
                Ok::<ElseBranch, eyre::Error>(ElseBranch::If(Box::new(parse_if_stmt(p)?)))
            } else {
                Ok::<ElseBranch, eyre::Error>(ElseBranch::Block(parse_block(p)?))
            }
        })
        .transpose()?;
    Ok(IfStmt {
        condition,
        then_block,
        else_branch,
    })
}

fn parse_expr(pair: Pair<Rule>) -> eyre::Result<Expression> {
    match pair.as_rule() {
        Rule::expr => parse_logic_or(pair.into_inner().next().unwrap()),
        _ => parse_logic_or(pair),
    }
}

fn parse_logic_or(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::logic_or {
        return parse_logic_and(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_logic_and(inner.next().unwrap())?;
    while let Some(_) = inner.next() {
        let right = parse_logic_and(inner.next().unwrap())?;
        expr = Expression::Binary(BinaryOp::LogicOr, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_logic_and(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::logic_and {
        return parse_bit_or(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_bit_or(inner.next().unwrap())?;
    while let Some(_) = inner.next() {
        let right = parse_bit_or(inner.next().unwrap())?;
        expr = Expression::Binary(BinaryOp::LogicAnd, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_bit_or(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::bit_or {
        return parse_bit_xor(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_bit_xor(inner.next().unwrap())?;
    while let Some(_) = inner.next() {
        let right = parse_bit_xor(inner.next().unwrap())?;
        expr = Expression::Binary(BinaryOp::BitOr, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_bit_xor(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::bit_xor {
        return parse_bit_and(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_bit_and(inner.next().unwrap())?;
    while let Some(_) = inner.next() {
        let right = parse_bit_and(inner.next().unwrap())?;
        expr = Expression::Binary(BinaryOp::BitXor, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_bit_and(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::bit_and {
        return parse_equality(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_equality(inner.next().unwrap())?;
    while let Some(_) = inner.next() {
        let right = parse_equality(inner.next().unwrap())?;
        expr = Expression::Binary(BinaryOp::BitAnd, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_equality(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::equality {
        return parse_comparison(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_comparison(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "==" => BinaryOp::Eq,
            "!=" => BinaryOp::Ne,
            _ => unreachable!(),
        };
        let right = parse_comparison(inner.next().unwrap())?;
        expr = Expression::Binary(op, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_comparison(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::comparison {
        return parse_shift(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_shift(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "<" => BinaryOp::Lt,
            ">" => BinaryOp::Gt,
            "<=" => BinaryOp::Le,
            ">=" => BinaryOp::Ge,
            _ => unreachable!(),
        };
        let right = parse_shift(inner.next().unwrap())?;
        expr = Expression::Binary(op, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_shift(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::shift {
        return parse_addition(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_addition(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "<<" => BinaryOp::Shl,
            ">>" => BinaryOp::Shr,
            _ => unreachable!(),
        };
        let right = parse_addition(inner.next().unwrap())?;
        expr = Expression::Binary(op, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_addition(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::addition {
        return parse_multiplication(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_multiplication(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            _ => unreachable!(),
        };
        let right = parse_multiplication(inner.next().unwrap())?;
        expr = Expression::Binary(op, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_multiplication(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::multiplication {
        return parse_unary(pair);
    }
    let mut inner = pair.into_inner();
    let mut expr = parse_unary(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Mod,
            _ => unreachable!(),
        };
        let right = parse_unary(inner.next().unwrap())?;
        expr = Expression::Binary(op, Box::new(expr), Box::new(right));
    }
    Ok(expr)
}

fn parse_unary(pair: Pair<Rule>) -> eyre::Result<Expression> {
    if pair.as_rule() != Rule::unary {
        return parse_primary(pair);
    }
    let inner = pair.into_inner();
    let mut ops = Vec::new();
    for next in inner {
        match next.as_rule() {
            Rule::neg => ops.push(UnaryOp::Neg),
            Rule::not => ops.push(UnaryOp::Not),
            Rule::bit_not => ops.push(UnaryOp::BitNot),
            Rule::deref => ops.push(UnaryOp::Deref),
            _ => {
                let mut expr = parse_primary(next)?;
                for op in ops.into_iter().rev() {
                    expr = Expression::Unary(op, Box::new(expr));
                }
                return Ok(expr);
            }
        }
    }
    eyre::bail!("unary missing primary expression")
}

fn parse_primary(pair: Pair<Rule>) -> eyre::Result<Expression> {
    match pair.as_rule() {
        Rule::primary => parse_primary(pair.into_inner().next().unwrap()),
        Rule::expr => parse_expr(pair),
        Rule::reserve_expr => {
            let mut inner = pair.into_inner();
            let ty = parse_type(inner.next().unwrap())?;
            let count_pair = inner.next().unwrap();
            let count = match count_pair.as_rule() {
                Rule::dec_literal => count_pair.as_str().parse()?,
                Rule::hex_literal => usize::from_str_radix(&count_pair.as_str()[2..], 16)?,
                _ => unreachable!("reserve count must be a literal"),
            };
            Ok(Expression::Reserve(ty, count))
        }
        Rule::call_expr => {
            let mut c_inner = pair.into_inner();
            let path = parse_path(c_inner.next().unwrap())?;
            let mut args = Vec::new();
            if let Some(args_pair) = c_inner.next() {
                for arg_pair in args_pair.into_inner() {
                    args.push(parse_expr(arg_pair)?);
                }
            }
            Ok(Expression::Call(path, args))
        }
        Rule::index_expr => {
            let mut idx_inner = pair.into_inner();
            let path = parse_path(idx_inner.next().unwrap())?;
            let idx = parse_expr(idx_inner.next().unwrap())?;
            Ok(Expression::Index(path, Box::new(idx)))
        }
        Rule::literal => {
            let l_inner = pair.into_inner().next().unwrap();
            match l_inner.as_rule() {
                Rule::dec_literal => {
                    Ok(Expression::Literal(Literal::Int(l_inner.as_str().parse()?)))
                }
                Rule::hex_literal => Ok(Expression::Literal(Literal::Uint(u32::from_str_radix(
                    &l_inner.as_str()[2..],
                    16,
                )?))),
                Rule::char_literal => Ok(Expression::Literal(Literal::Char(
                    l_inner.as_str().as_bytes()[1],
                ))),
                Rule::string_literal => Ok(Expression::Literal(Literal::String(
                    l_inner.as_str()[1..l_inner.as_str().len() - 1].to_string(),
                ))),
                Rule::bool_literal => Ok(Expression::Literal(Literal::Bool(
                    l_inner.as_str() == "true",
                ))),
                _ => unreachable!("parse_literal: {:?}", l_inner.as_rule()),
            }
        }
        Rule::path => Ok(Expression::Path(parse_path(pair)?)),
        _ => unreachable!("parse_primary: {:?}", pair.as_rule()),
    }
}

fn parse_type(pair: Pair<Rule>) -> eyre::Result<Type> {
    match pair.as_str() {
        "i32" => Ok(Type::I32),
        "u32" => Ok(Type::U32),
        "i8" => Ok(Type::I8),
        "u8" => Ok(Type::U8),
        "cstr" => Ok(Type::Cstr),
        "bool" => Ok(Type::Bool),
        _ => unreachable!("parse_type: {:?}", pair.as_rule()),
    }
}
