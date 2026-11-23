//@ run-pass
//! Tests that the stable MIR API lowers an async closure's type and its body.

//@ ignore-stage1
//@ ignore-cross-compile
//@ ignore-remote
//@ edition: 2024

#![feature(rustc_private)]

extern crate rustc_middle;

extern crate rustc_driver;
extern crate rustc_interface;
#[macro_use]
extern crate rustc_public;

use std::io::Write;
use std::ops::ControlFlow;

use rustc_public::mir::{AggregateKind, Rvalue, StatementKind};
use rustc_public::CrateDef;
use rustc_public::ty::{RigidTy, TyKind};

const CRATE_NAME: &str = "crate_coroutine_closure";

fn test_coroutine_closure() -> ControlFlow<()> {
    let mut closure_types = 0;
    let mut aggregates = 0;
    for item in rustc_public::all_local_items() {
        if let TyKind::RigidTy(RigidTy::CoroutineClosure(def, _)) = item.ty().kind() {
            assert_eq!(def.name(), "crate_coroutine_closure::main::{closure#0}");
            closure_types += 1;
        }
        let Some(body) = item.body() else { continue };
        // The body that builds the closure has a local of its type.
        for local in body.locals() {
            local.ty.kind();
        }
        for block in &body.blocks {
            for statement in &block.statements {
                if let StatementKind::Assign(
                    _,
                    Rvalue::Aggregate(AggregateKind::CoroutineClosure(..), _),
                ) = &statement.kind
                {
                    aggregates += 1;
                }
            }
        }
    }
    assert_eq!(closure_types, 1);
    assert_eq!(aggregates, 1);
    ControlFlow::Continue(())
}

fn main() {
    let path = "coroutine_closure.rs";
    generate_input(&path).unwrap();
    let args = &[
        "rustc".to_string(),
        "-Cpanic=abort".to_string(),
        "--edition=2024".to_string(),
        "--crate-name".to_string(),
        CRATE_NAME.to_string(),
        path.to_string(),
    ];
    run!(args, test_coroutine_closure).unwrap();
}

fn generate_input(path: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    write!(
        file,
        r#"
        fn main() {{
            let add = async |x: i32| x + 1;
            let _ = add(1);
        }}
    "#
    )?;
    Ok(())
}
