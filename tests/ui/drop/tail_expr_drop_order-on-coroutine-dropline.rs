// Regression test: with `tail_expr_drop_order` enabled, a coroutine scope that has both an
// async-drop local and a lint-only drop used to leave MIR building's coroutine drop-tree cursor one
// node behind, tripping a debug assertion in `build_scope_drops`.
//@ edition: 2021
//@ check-pass
//@ compile-flags: -W rust-2024-compatibility

pub struct S(String);

async fn conn(_: &String) -> Result<S, String> {
    Ok(S(String::new()))
}

pub async fn g(p: String) -> S {
    conn(&p).await.expect("x")
}

fn main() {}
