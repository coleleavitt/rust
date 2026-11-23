//@ run-pass
//! Tests that the stable MIR API lowers an async closure's type, its signature and its body.

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
use rustc_public::ty::{IntTy, RigidTy, TyKind};

const CRATE_NAME: &str = "crate_coroutine_closure";

fn test_coroutine_closure() -> ControlFlow<()> {
    let mut closure_types = 0;
    let mut aggregates = 0;
    for item in rustc_public::all_local_items() {
        let kind = item.ty().kind();
        if let TyKind::RigidTy(RigidTy::CoroutineClosure(def, _)) = &kind {
            assert_eq!(def.name(), "crate_coroutine_closure::main::{closure#0}");
            assert!(kind.is_coroutine_closure());
            assert!(!kind.is_closure());
            check_signature(&kind);
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

/// Calling `async |x: i32| x + 1` takes the tupled `(i32,)` and returns a coroutine.
fn check_signature(kind: &TyKind) {
    let sig = kind.fn_sig().expect("an async closure has a signature").skip_binder();
    let [input] = sig.inputs() else { panic!("expected one tupled input, got {:?}", sig.inputs()) };
    let TyKind::RigidTy(RigidTy::Tuple(fields)) = input.kind() else {
        panic!("expected a tuple, got {input:?}")
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].kind(), TyKind::RigidTy(RigidTy::Int(IntTy::I32)));
    assert!(sig.output().kind().is_coroutine(), "{:?}", sig.output());
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
