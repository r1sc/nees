fn main() {
    cc::Build::new()
        .file("src/fake6502.c")
        .compile("fake6502");
}