use vergen;

fn main() {
    vergen::EmitBuilder::builder()
        .git_describe(false, false, None)
        .cargo_debug()
        .fail_on_error()
        .emit_and_set()
        .expect("Build Error");
}
