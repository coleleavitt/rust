// Regression test for #142155: the async-block variant of
// `tail_expr_drop_order-on-coroutine-dropline.rs`.
//@ edition: 2021
//@ check-pass

#![warn(tail_expr_drop_order)]
use core::future::Future;

fn f() -> impl Future<Output = Option<String>> {
    async { Some("nope".into()) }
}

fn main() {}
