## **Under development!**

You can run PriEco on your own server. This is great if you want to help us crawl the web and/or if you want to run your own version of PriEco

The first step is to clone PriEco

```bash
git clone https://codeberg.org/JojoYou/PriEco.git
```

and get to PriEco directory

```bash
cd PriEco/
```

Then you can compile PriEco as a binary or run a docker image

## Compile

### NVIDIA (For running PriEco as search enigne)

```rust
RUSTFLAGS='-C target-cpu=native' cargo build --profile release-dev --features cuda
```

### ARM

```rust
RUSTFLAGS="-C target-cpu=cortex-a72" JEMALLOC_SYS_WITH_LG_PAGE=16 CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++ cargo build --target aarch64-unknown-linux-gnu --release
```
